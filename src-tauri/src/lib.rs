use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Mutex, time::{SystemTime, UNIX_EPOCH}};
use tauri::{menu::{Menu, MenuItem}, tray::TrayIconBuilder, Emitter, Manager, State, WindowEvent};
use keyring::Entry;

struct AppState { db: Mutex<Connection> }
const OPENAI_KEY_SERVICE: &str = "Mouse AI Studio";
const OPENAI_KEY_USER: &str = "openai-api-key";

#[tauri::command]
fn save_openai_api_key(api_key: String) -> Result<(), String> {
    let key = api_key.trim();

    if key.is_empty() || key.len() > 512 {
        return Err("Invalid API key".into());
    }

    let entry = Entry::new(OPENAI_KEY_SERVICE, OPENAI_KEY_USER)
        .map_err(|e| e.to_string())?;

    entry.set_password(key).map_err(|e| e.to_string())
}

#[tauri::command]
fn has_openai_api_key() -> Result<bool, String> {
    let entry = Entry::new(OPENAI_KEY_SERVICE, OPENAI_KEY_USER)
        .map_err(|e| e.to_string())?;

    Ok(entry.get_password().is_ok())
}

#[tauri::command]
fn delete_openai_api_key() -> Result<(), String> {
    let entry = Entry::new(OPENAI_KEY_SERVICE, OPENAI_KEY_USER)
        .map_err(|e| e.to_string())?;

    entry.delete_credential().map_err(|e| e.to_string())
}
#[tauri::command]
fn ask_openai(message: String) -> Result<String, String> {
    let message = message.trim();

    if message.is_empty() || message.len() > 12000 {
        return Err("Message must contain 1–12000 characters".into());
    }

    let entry = Entry::new(OPENAI_KEY_SERVICE, OPENAI_KEY_USER)
        .map_err(|e| e.to_string())?;

    let api_key = entry.get_password()
        .map_err(|_| "Please configure your OpenAI API key first".to_string())?;

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .post("https://api.openai.com/v1/responses")
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "model": "gpt-6.2-sol",
            "instructions": "You are Mouse AI Studio's assistant. Help users understand mouse hardware, settings, diagnostics and troubleshooting. Never invent hardware readings or DPI values. Clearly distinguish measured data from estimates.",
            "input": message
        }))
        .send()
        .map_err(|e| format!("OpenAI connection failed: {e}"))?;

    let status = response.status();
    let body: serde_json::Value = response.json()
        .map_err(|e| format!("Could not read OpenAI response: {e}"))?;

    if !status.is_success() {
        let error = body["error"]["message"]
            .as_str()
            .unwrap_or("OpenAI request failed");
        return Err(error.to_string());
    }

    for item in body["output"].as_array().into_iter().flatten() {
        for content in item["content"].as_array().into_iter().flatten() {
            if content["type"] == "output_text" {
                if let Some(text) = content["text"].as_str() {
                    return Ok(text.to_string());
                }
            }
        }
    }

    Err("OpenAI returned no text response".into())
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct PointingDevice {
    id: String,
    name: String,
    manufacturer: Option<String>,
    vid: Option<String>,
    pid: Option<String>,
    version: Option<String>,
    hardware_ids: Vec<String>,
    connection: String,
    status: String,
    interface_path: String,
    source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MouseProfile { name: String, dpi: u32, saved_at: i64 }

#[tauri::command]
fn list_pointing_devices() -> Result<Vec<PointingDevice>, String> {

    #[cfg(windows)]
    { return windows_hardware::enumerate(); }
    #[cfg(not(windows))]
    { Ok(Vec::new()) }
}
    #[cfg(windows)]
#[tauri::command]
fn capture_hs6209_report() -> Result<Vec<u8>, String> {
    windows_hardware::capture_hs6209_report()
}

#[tauri::command]
fn list_profiles(state: State<'_, AppState>) -> Result<Vec<MouseProfile>, String> {
    let db = state.db.lock().map_err(|_| "Database lock is unavailable".to_string())?;
    let mut stmt = db.prepare("SELECT name, dpi, saved_at FROM profiles ORDER BY name")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| Ok(MouseProfile { name: r.get(0)?, dpi: r.get(1)?, saved_at: r.get(2)? }))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
fn save_profile(state: State<'_, AppState>, name: String, dpi: u32) -> Result<(), String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 40 { return Err("Profile name must contain 1–40 characters".into()); }
    if !(100..=20_000).contains(&dpi) { return Err("Configured DPI must be between 100 and 20,000".into()); }
    let db = state.db.lock().map_err(|_| "Database lock is unavailable".to_string())?;
    let saved_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    db.execute("INSERT INTO profiles(name,dpi,saved_at) VALUES(?1,?2,?3) ON CONFLICT(name) DO UPDATE SET dpi=excluded.dpi,saved_at=excluded.saved_at", params![name, dpi, saved_at]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    let db = state.db.lock().map_err(|_| "Database lock is unavailable".to_string())?;
    db.query_row("SELECT value FROM settings WHERE key=?1", [key], |row| row.get(0)).optional().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    if key.is_empty() || key.len() > 80 || value.len() > 4000 { return Err("Setting key or value is outside the allowed size".into()); }
    let db = state.db.lock().map_err(|_| "Database lock is unavailable".to_string())?;
    db.execute("INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key,value]).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let mut data_dir: PathBuf = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            data_dir.push("mouse-ai-studio.sqlite3");
            let db = Connection::open(data_dir)?;
            db.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE IF NOT EXISTS profiles(name TEXT PRIMARY KEY NOT NULL,dpi INTEGER NOT NULL,saved_at INTEGER NOT NULL); CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY NOT NULL,value TEXT NOT NULL);")?;
            app.manage(AppState { db: Mutex::new(db) });

            let show = MenuItem::with_id(app, "show", "Show Mouse AI Studio", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let mut tray = TrayIconBuilder::with_id("mouse-ai-studio-tray")
                .tooltip("Mouse AI Studio");
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => if let Some(window) = app.get_webview_window("main") { let _ = window.show(); let _ = window.set_focus(); },
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            #[cfg(windows)]
            start_device_watcher(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
    list_pointing_devices,
    capture_hs6209_report,
    list_profiles,
    save_profile,
    get_setting,
    set_setting,
    save_openai_api_key,
    has_openai_api_key,
    delete_openai_api_key,
    ask_openai
])
        .run(tauri::generate_context!())
        .expect("error while running Mouse AI Studio");
}

#[cfg(windows)]
fn start_device_watcher(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        use std::{collections::BTreeSet, thread, time::Duration};
        let mut previous: BTreeSet<String> = windows_hardware::enumerate().unwrap_or_default().into_iter().map(|d| d.id).collect();
        loop {
            thread::sleep(Duration::from_secs(2));
            if let Ok(devices) = windows_hardware::enumerate() {
                let current: BTreeSet<String> = devices.iter().map(|d| d.id.clone()).collect();
                if current != previous {
                    previous = current;
                    let _ = app.emit("devices-changed", devices);
                }
            }
        }
    });
}

#[cfg(windows)]
mod windows_hardware {
    use super::PointingDevice;
    use std::{ffi::c_void, mem::size_of, ptr};

    type Handle = *mut c_void;
    type Bool = i32;
    #[repr(C)] #[derive(Clone, Copy, Default)] struct Guid { d1:u32,d2:u16,d3:u16,d4:[u8;8] }
    #[repr(C)] #[derive(Clone, Copy)] struct RawInputDeviceList { device:Handle, kind:u32 }
    #[repr(C)] #[derive(Clone, Copy)] struct DeviceInterfaceData { cb_size:u32, class_guid:Guid, flags:u32, reserved:usize }
    #[repr(C)] #[derive(Clone, Copy)] struct DeviceInfoData { cb_size:u32, class_guid:Guid, dev_inst:u32, reserved:usize }
    #[repr(C)] struct HidAttributes { size:u32, vendor_id:u16, product_id:u16, version_number:u16 }
    #[repr(C)]
struct Overlapped {
    internal: usize,
    internal_high: usize,
    offset: u32,
    offset_high: u32,
    event: Handle,
}
    #[repr(C)]
struct HidpCaps {
    usage: u16,
    usage_page: u16,
    input_report_byte_length: u16,
    output_report_byte_length: u16,
    feature_report_byte_length: u16,
    reserved: [u16; 17],
    number_link_collection_nodes: u16,
    number_input_button_caps: u16,
    number_input_value_caps: u16,
    number_input_data_indices: u16,
    number_output_button_caps: u16,
    number_output_value_caps: u16,
    number_output_data_indices: u16,
    number_feature_button_caps: u16,
    number_feature_value_caps: u16,
    number_feature_data_indices: u16,
}

    #[link(name="hid")]
    extern "system" {
        fn HidD_GetHidGuid(guid:*mut Guid);
        fn HidD_GetAttributes(device:Handle, attributes:*mut HidAttributes)->Bool;
        fn HidD_GetManufacturerString(device:Handle, buffer:*mut c_void, length:u32)->Bool;
        fn HidD_GetProductString(device:Handle, buffer:*mut c_void, length:u32)->Bool;
        fn HidD_GetPreparsedData(device: Handle, data: *mut *mut c_void) -> Bool;
        fn HidD_FreePreparsedData(data: *mut c_void) -> Bool;
    }

#[repr(C)]
#[derive(Clone, Copy)]
struct DevPropKey {
    fmtid: Guid,
    pid: u32,
}

#[link(name="cfgmgr32")]
extern "system" {
    fn CM_Get_Parent(
        parent: *mut u32,
        device_instance: u32,
        flags: u32,
    ) -> u32;

    fn CM_Get_DevNode_PropertyW(
        device_instance: u32,
        property_key: *const DevPropKey,
        property_type: *mut u32,
        property_buffer: *mut u8,
        property_buffer_size: *mut u32,
        flags: u32,
    ) -> u32;
}

    #[link(name="setupapi")]
    extern "system" {
        fn SetupDiGetClassDevsW(class_guid:*const Guid, enumerator:*const u16, parent:Handle, flags:u32)->Handle;
        fn SetupDiEnumDeviceInterfaces(set:Handle, device_info:*const DeviceInfoData, class_guid:*const Guid, index:u32, interface_data:*mut DeviceInterfaceData)->Bool;
        fn SetupDiGetDeviceInterfaceDetailW(set:Handle, interface_data:*mut DeviceInterfaceData, detail:*mut c_void, detail_size:u32, required_size:*mut u32, device_info:*mut DeviceInfoData)->Bool;
        fn SetupDiGetDeviceRegistryPropertyW(set:Handle, device_info:*mut DeviceInfoData, property:u32, property_type:*mut u32, buffer:*mut u8, buffer_size:u32, required_size:*mut u32)->Bool;
        fn SetupDiDestroyDeviceInfoList(set:Handle)->Bool;
    }
    #[link(name="user32")]
    extern "system" {
        fn GetRawInputDeviceList(list:*mut RawInputDeviceList, count:*mut u32, size:u32)->u32;
        fn GetRawInputDeviceInfoW(device:Handle, command:u32, data:*mut c_void, size:*mut u32)->u32;
    }
    #[link(name="kernel32")]
    extern "system" {
        fn CreateFileW(path:*const u16, access:u32, share:u32, security:*mut c_void, creation:u32, flags:u32, template:Handle)->Handle;
        fn ReadFile(
    handle: Handle,
    buffer: *mut c_void,
    bytes_to_read: u32,
    bytes_read: *mut u32,
    overlapped: *mut c_void,
) -> Bool;
fn CreateEventW(
    security: *mut c_void,
    manual_reset: Bool,
    initial_state: Bool,
    name: *const u16,
) -> Handle;

fn WaitForSingleObject(handle: Handle, milliseconds: u32) -> u32;

fn GetOverlappedResult(
    handle: Handle,
    overlapped: *mut Overlapped,
    bytes_transferred: *mut u32,
    wait: Bool,
) -> Bool;

fn CancelIoEx(handle: Handle, overlapped: *mut Overlapped) -> Bool;
        fn CloseHandle(handle:Handle)->Bool;
        fn GetLastError() -> u32;
    }

    const ERROR:u32=u32::MAX; const RIM_TYPEMOUSE:u32=0;
    const RIDI_DEVICENAME:u32=0x20000007; const DIGCF_PRESENT:u32=0x2; const DIGCF_DEVICEINTERFACE:u32=0x10;
    const SPDRP_DEVICEDESC:u32=0; const SPDRP_HARDWAREID:u32=1; const SPDRP_MFG:u32=11; const SPDRP_FRIENDLYNAME:u32=12;
    const FILE_SHARE_READ:u32=1; const FILE_SHARE_WRITE:u32=2; const OPEN_EXISTING:u32=3; const INVALID_HANDLE:Handle=-1isize as Handle;

    pub fn enumerate()->Result<Vec<PointingDevice>,String>{
        let raw_paths=raw_mouse_paths()?;
        if raw_paths.is_empty(){return Ok(Vec::new())}
        let interfaces=setup_hid_interfaces()?;
        let mut result=Vec::new();
        for path in raw_paths {
            
            let info = interfaces.iter().find(|entry| {
    let raw_id = path.split("#{").next().unwrap_or(&path);
    let setup_id = entry.path.split("#{").next().unwrap_or(&entry.path);
    setup_id.eq_ignore_ascii_case(raw_id)
});
            let properties=info.map(|v|&v.properties);
            let hardware_ids=properties.and_then(|p|p.get(&SPDRP_HARDWAREID)).cloned().unwrap_or_default();
            
            let vid = parse_id(&hardware_ids, "VID_")
                .or_else(|| parse_id(std::slice::from_ref(&path), "VID_"));

            let pid = parse_id(&hardware_ids, "PID_")
                .or_else(|| parse_id(std::slice::from_ref(&path), "PID_"));

            let (hid_manufacturer,hid_product,hid_version)=read_hid_strings(&path);
            let pnp_name=properties.and_then(|p|p.get(&SPDRP_FRIENDLYNAME).or_else(||p.get(&SPDRP_DEVICEDESC))).and_then(|v|v.first()).cloned();
            let parent_name = info.and_then(|entry| parent_usb_description(entry.dev_inst));
            let pnp_mfg=properties.and_then(|p|p.get(&SPDRP_MFG)).and_then(|v|v.first()).cloned();
            let manufacturer=hid_manufacturer.filter(|s|!s.is_empty()).or_else(||pnp_mfg.filter(|s|!(s.eq_ignore_ascii_case("Microsoft")&&vid.as_deref()!=Some("045E"))));
            
            let product_name = hid_product
    .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("HID-compliant mouse"))
    .or(pnp_name.filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("HID-compliant mouse")))
    .or(parent_name.filter(|s| !s.is_empty()));
            let name=product_name.unwrap_or_else(||"HID-compliant mouse".into());
            let upper=path.to_ascii_uppercase();
            let connection=if upper.contains("BTH")||upper.contains("BLUETOOTH"){"Bluetooth"}else if upper.contains("USB"){"USB/HID"}else{"HID (transport unknown)"};
            result.push(PointingDevice{id:path.clone(),name,manufacturer,vid,pid,version:hid_version,hardware_ids,connection:connection.into(),status:"Connected".into(),interface_path:path,source:"Windows Raw Input + SetupAPI/HID (present mouse collections)".into()});
        }
        result.sort_by(|a,b|a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
        Ok(result)
    }

    pub fn capture_hs6209_report() -> Result<Vec<u8>, String> {
    unsafe {
        let interfaces = setup_hid_interfaces()?;

        let path = interfaces.iter().find(|item| {
    let p = item.path.to_ascii_uppercase();
    p.contains("VID_062A")
        && p.contains("PID_4C01")
        && p.contains("MI_01")
        && p.contains("COL01")
})
            .map(|item| item.path.clone())
            .ok_or("HS6209 vendor interface not found")?;

        let wide: Vec<u16> = path.encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let device = CreateFileW(
            wide.as_ptr(),
            0x80000000,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            ptr::null_mut(),
            OPEN_EXISTING,
            0x40000000,
            ptr::null_mut(),
        );

        if device == INVALID_HANDLE {
            return Err(format!(
    "Could not open HS6209 for reading. Windows error code: {}",
    GetLastError()
));
        }

        let event = CreateEventW(
            ptr::null_mut(), 1, 0, ptr::null(),
        );

        if event.is_null() {
            let _ = CloseHandle(device);
            return Err("Could not create read event".into());
        }

        let mut buffer = [0u8; 8];
        let mut bytes_read = 0u32;
        let mut overlapped: Overlapped = std::mem::zeroed();
        overlapped.event = event;

        let started = ReadFile(
            device,
            buffer.as_mut_ptr().cast(),
            8,
            &mut bytes_read,
            &mut overlapped as *mut Overlapped as *mut c_void,
        );

        let result = if started != 0 {
            Ok(bytes_read)
        } else {
            let wait = WaitForSingleObject(event, 1000);

            if wait == 0 {
                if GetOverlappedResult(
                    device, &mut overlapped, &mut bytes_read, 0
                ) != 0 {
                    Ok(bytes_read)
                } else {
                    Err("HID report read failed".to_string())
                }
            } else {
                let _ = CancelIoEx(device, &mut overlapped);
                let _ = WaitForSingleObject(event, u32::MAX);
                Err("No HID report received within 1 second".to_string())
            }
        };

        let _ = CloseHandle(event);
        let _ = CloseHandle(device);

        result.map(|count| buffer[..(count as usize).min(8)].to_vec())
    }
}
    fn raw_mouse_paths()->Result<Vec<String>,String>{unsafe{
        let item_size=size_of::<RawInputDeviceList>() as u32; let mut count=0;
        let first=GetRawInputDeviceList(ptr::null_mut(),&mut count,item_size);if first==ERROR{return Err("GetRawInputDeviceList could not count mouse devices".into())}if count==0{return Ok(Vec::new())}
        let mut entries=vec![RawInputDeviceList{device:ptr::null_mut(),kind:0};count as usize];
        let read=GetRawInputDeviceList(entries.as_mut_ptr(),&mut count,item_size); if read==ERROR{return Err("GetRawInputDeviceList failed".into())}
        entries.truncate(read as usize); let mut paths: Vec<String> = Vec::new();
        for entry in entries.into_iter().filter(|v|v.kind==RIM_TYPEMOUSE){let mut chars=0;let _=GetRawInputDeviceInfoW(entry.device,RIDI_DEVICENAME,ptr::null_mut(),&mut chars);if chars==0||chars==ERROR{continue}let mut buffer=vec![0u16;chars as usize+1];let got=GetRawInputDeviceInfoW(entry.device,RIDI_DEVICENAME,buffer.as_mut_ptr().cast(),&mut chars);if got==ERROR{continue}let length=(got as usize).min(buffer.len());let name=String::from_utf16_lossy(&buffer[..length]).trim_end_matches('\0').to_string();if !name.is_empty()&&!paths.iter().any(|p|p.eq_ignore_ascii_case(&name)){paths.push(name)}}
        Ok(paths)
    }}

    struct HidInterface {
    path: String,
    dev_inst: u32,
    properties: std::collections::HashMap<u32, Vec<String>>,
}
    fn setup_hid_interfaces()->Result<Vec<HidInterface>,String>{unsafe{
        let mut guid=Guid::default();HidD_GetHidGuid(&mut guid);
        let set=SetupDiGetClassDevsW(&guid,ptr::null(),ptr::null_mut(),DIGCF_PRESENT|DIGCF_DEVICEINTERFACE);
        if set==INVALID_HANDLE{return Err("SetupDiGetClassDevsW could not enumerate present HID interfaces".into())}
        let mut entries=Vec::new();let mut index=0u32;
        loop{
            let mut interface=DeviceInterfaceData{cb_size:size_of::<DeviceInterfaceData>() as u32,class_guid:Guid::default(),flags:0,reserved:0};
            if SetupDiEnumDeviceInterfaces(set,ptr::null(),&guid,index,&mut interface)==0{break}
            index+=1;let mut required=0u32;let mut devinfo=DeviceInfoData{cb_size:size_of::<DeviceInfoData>() as u32,class_guid:Guid::default(),dev_inst:0,reserved:0};
            let _=SetupDiGetDeviceInterfaceDetailW(set,&mut interface,ptr::null_mut(),0,&mut required,&mut devinfo);if required<6{continue}
            let mut detail=vec![0u8;required as usize];let detail_cb_size=if size_of::<usize>()==8{8u32}else{6u32};ptr::write_unaligned(detail.as_mut_ptr().cast::<u32>(),detail_cb_size);
            if SetupDiGetDeviceInterfaceDetailW(set,&mut interface,detail.as_mut_ptr().cast(),required,&mut required,&mut devinfo)==0{continue}
            let wide=detail.as_ptr().add(4).cast::<u16>();let max=(detail.len().saturating_sub(4))/2;let mut len=0;while len<max&&ptr::read_unaligned(wide.add(len))!=0{len+=1}let path=String::from_utf16_lossy(std::slice::from_raw_parts(wide,len));
            let mut properties=std::collections::HashMap::new();for property in [SPDRP_DEVICEDESC,SPDRP_HARDWAREID,SPDRP_MFG,SPDRP_FRIENDLYNAME]{let vals=read_device_property(set,&mut devinfo,property);if !vals.is_empty(){properties.insert(property,vals);}}
            entries.push(HidInterface {
    path,
    dev_inst: devinfo.dev_inst,
    properties,
});
        }
        let _=SetupDiDestroyDeviceInfoList(set);Ok(entries)
    }}

    unsafe fn read_device_property(set:Handle,info:*mut DeviceInfoData,property:u32)->Vec<String>{
        let mut ty=0u32;let mut required=0u32;let mut buffer=vec![0u16;2048];
        if SetupDiGetDeviceRegistryPropertyW(set,info,property,&mut ty,buffer.as_mut_ptr().cast(),(buffer.len()*2) as u32,&mut required)==0{return Vec::new()}
        let count=((required as usize)/2).min(buffer.len());let slice=&buffer[..count];
        if property==SPDRP_HARDWAREID{slice.split(|v|*v==0).take_while(|s|!s.is_empty()).map(String::from_utf16_lossy).collect()}else{let end=slice.iter().position(|v|*v==0).unwrap_or(slice.len());let text=String::from_utf16_lossy(&slice[..end]).trim().to_string();if text.is_empty(){Vec::new()}else{vec![text]}}
    }


fn parent_usb_description(device_instance: u32) -> Option<String> {
    unsafe {
        const BUS_REPORTED_DESC: DevPropKey = DevPropKey {
            fmtid: Guid {
                d1: 0x540B947E,
                d2: 0x8B40,
                d3: 0x45BC,
                d4: [0xA8, 0xA2, 0x6A, 0x0B, 0x89, 0x4C, 0xBD, 0xA2],
            },
            pid: 4,
        };

        let mut current = device_instance;

        for _ in 0..8 {
            let mut property_type = 0u32;
            let mut buffer_size = 0u32;

            let first = CM_Get_DevNode_PropertyW(
                current,
                &BUS_REPORTED_DESC,
                &mut property_type,
                ptr::null_mut(),
                &mut buffer_size,
                0,
            );

            if first == 0x1A && buffer_size > 0 {
                let mut buffer = vec![0u8; buffer_size as usize];

                if CM_Get_DevNode_PropertyW(
                    current,
                    &BUS_REPORTED_DESC,
                    &mut property_type,
                    buffer.as_mut_ptr(),
                    &mut buffer_size,
                    0,
                ) == 0 && property_type == 0x12 {
                    let wide = std::slice::from_raw_parts(
                        buffer.as_ptr().cast::<u16>(),
                        buffer_size as usize / 2,
                    );

                    let end = wide.iter().position(|v| *v == 0).unwrap_or(wide.len());
                    let description =
                        String::from_utf16_lossy(&wide[..end]).trim().to_string();

                    if !description.is_empty() {
                        return Some(description);
                    }
                }
            }

            let mut parent = 0u32;

            if CM_Get_Parent(&mut parent, current, 0) != 0 {
                break;
            }

            current = parent;
        }

        None
    }
}

    fn parse_id(ids:&[String],key:&str)->Option<String>{
        let text=ids.iter().find_map(|id|{let up=id.to_ascii_uppercase();let start=up.find(key)?+key.len();let value:String=up[start..].chars().take_while(|c|c.is_ascii_hexdigit()).take(4).collect();(value.len()==4).then_some(value)})?;Some(text)
    }

    fn read_hid_strings(path:&str)->(Option<String>,Option<String>,Option<String>){unsafe{
        let wide:Vec<u16>=path.encode_utf16().chain(std::iter::once(0)).collect();let device=CreateFileW(wide.as_ptr(),0x8000_0000,FILE_SHARE_READ|FILE_SHARE_WRITE,ptr::null_mut(),OPEN_EXISTING,0,ptr::null_mut());
        if device==INVALID_HANDLE{return(None,None,None)}
        let mut attributes=HidAttributes{size:size_of::<HidAttributes>() as u32,vendor_id:0,product_id:0,version_number:0};let version=if HidD_GetAttributes(device,&mut attributes)!=0{Some(format!("{:04X}",attributes.version_number))}else{None};
        let manufacturer=read_hid_string(device,HidD_GetManufacturerString);let product=read_hid_string(device,HidD_GetProductString);let _=CloseHandle(device);(manufacturer,product,version)
    }}
    unsafe fn read_hid_string(device:Handle,read:unsafe extern "system" fn(Handle,*mut c_void,u32)->Bool)->Option<String>{
        let mut buffer=[0u16;256];if read(device,buffer.as_mut_ptr().cast(),(buffer.len()*2) as u32)==0{return None}let end=buffer.iter().position(|v|*v==0).unwrap_or(buffer.len());let value=String::from_utf16_lossy(&buffer[..end]).trim().to_string();(!value.is_empty()).then_some(value)
    }
}
