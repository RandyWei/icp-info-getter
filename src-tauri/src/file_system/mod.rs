use std::{fs::File, io};

#[tauri::command]
pub fn save_file_to_zip(zip_path: &str, file_path: &str) -> Result<(), String> {
    let mut file = match File::create(file_path) {
        Ok(f) => f,
        Err(_) => return Err("创建文件失败".into()),
    };

    let mut zip_file = match File::open(zip_path) {
        Ok(f) => f,
        Err(_) => return Err("打开文件失败".into()),
    };

    //拷贝文件
    match io::copy(&mut zip_file, &mut file) {
        Ok(_) => {}
        Err(_) => return Err("复制失败".into()),
    };

    Ok(())
}
