use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
    process::Command,
};

use base64::{engine::general_purpose, Engine};
use plist::Value;
use serde::Serialize;
use zip::ZipArchive;

#[derive(Debug, Serialize)]
pub struct IcpResult {
    name: String,
    bundle_id: String,
    icon: String,
    sha1: String,
    modulus: String,
}

#[tauri::command]
pub fn parse(ipa_path: &str, cache_path: &str) -> Result<IcpResult, String> {
    //解压文件
    let extra_app_path = extra(Path::new(ipa_path), Path::new(cache_path))?;

    let plist_result = parse_plist(&extra_app_path)?;
    let codesign0_path = extract_certificates(&extra_app_path)?;

    let openssl_result = exec_openssl(&codesign0_path)?;

    Ok(IcpResult {
        name: plist_result.0,
        bundle_id: plist_result.1,
        icon: plist_result.2,
        sha1: openssl_result.0,
        modulus: openssl_result.1,
    })
}

/**
 * 解压文件
 */
fn extra(zip_file: &Path, target: &Path) -> Result<String, String> {
    //获取压缩包名字
    let zip_file_name = match zip_file.file_name() {
        Some(name) => name,
        None => return Err("文件名字获取失败".into()),
    };

    //在缓存路径上拼接上压缩包名字
    let binding = target.join(zip_file_name);
    let target = binding.as_path();

    //从路径打开压缩包文件
    let achieve = match File::open(zip_file) {
        Ok(a) => a,
        Err(_) => return Err("文件打开失败".into()),
    };

    //创建一个压缩包对象
    let mut zip = match ZipArchive::new(achieve) {
        Ok(z) => z,
        Err(_) => return Err("创建压缩包文件失败".into()),
    };

    if target.exists() {
        match fs::remove_dir_all(target) {
            Ok(_) => {}
            Err(_) => return Err("删除失败".into()),
        };
    }

    //递归创建文件夹
    match fs::create_dir_all(target) {
        Ok(_) => {}
        Err(_) => return Err("新建文件夹失败".into()),
    };

    for i in 0..zip.len() {
        //取出zip中的文件
        let mut file = zip.by_index(i).unwrap();

        let outpath = match file.enclosed_name() {
            Some(path) => Path::join(target, path.to_owned()),
            None => continue,
        };

        //打印注释
        {
            let comment = file.comment();
            if !comment.is_empty() {
                println!("File {} comment:{}", i, comment);
            }
        }

        let file_name = &*file.name();

        //只解压Payload目录
        if !file_name.contains("Payload") {
            continue;
        }

        if file_name.ends_with("/") {
            match fs::create_dir_all(&outpath) {
                Ok(_) => {}
                Err(_) => return Err("创建目录失败".into()),
            };
        } else {
            // println!(
            //     "File {} extracted to \"{}\" ({})bytes",
            //     i,
            //     outpath.display(),
            //     file.size()
            // );

            //逐级创建目录
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    match fs::create_dir_all(&p) {
                        Ok(_) => {}
                        Err(_) => return Err("创建目录失败".into()),
                    };
                }
            }

            //创建文件
            let mut out_file = match File::create(&outpath) {
                Ok(f) => f,
                Err(_) => return Err("创建文件失败".into()),
            };
            //拷贝文件
            match io::copy(&mut file, &mut out_file) {
                Ok(_) => {}
                Err(_) => return Err("复制失败".into()),
            };
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                match fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)) {
                    Ok(_) => {}
                    Err(_) => {
                        return Err("设置权限失败".into());
                    }
                };
            }
        }
    }

    let path = target.join("Payload");
    let mut app_path = String::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let file_path = entry.path();
                if file_path.is_dir()
                    && file_path
                        .extension()
                        .map(|ext| ext == ".app" || ext == "app")
                        .unwrap_or(false)
                {
                    app_path = file_path.as_path().display().to_string();
                }
            }
        }
    }

    Ok(app_path)
}

/**
 * 解压证书
 *
 * 其实就是执行命令```codesign -d --extract-certificates Runner.app```
 */
fn extract_certificates(app_path: &str) -> Result<String, String> {
    let mut cmd = Command::new("codesign");
    let path = match Path::new(app_path).parent() {
        Some(p) => p,
        None => {
            return Err("解析目录失败".into());
        }
    };
    cmd.current_dir(path)
        .arg("-d")
        .arg("--extract-certificates")
        .arg(app_path);

    let output = match cmd.output() {
        Ok(o) => o,
        Err(_) => {
            return Err("执行脚本失败".into());
        }
    };

    let err_str = String::from_utf8(output.stderr).unwrap();
    // let output_str = String::from_utf8(output.stdout).unwrap();

    // println!("执行结果1{}", output_str);
    //No such file or directory
    // println!("执行结果2{}", err_str);

    //执行结果包含以下字符串表示成功（未验证的逻辑）
    if err_str.contains("Executable=") {
        Ok(path.join("codesign0").as_path().display().to_string())
    } else {
        Err("好像执行失败了".into())
    }
}

/**
 * 执行openssl
 *
 * ```openssl x509 -fingerprint -sha1 -modulus -text -noout -in codesign0```
 */
fn exec_openssl(codesign0_path: &str) -> Result<(String, String), String> {
    let mut cmd = Command::new("openssl");

    cmd.arg("x509")
        .arg("-fingerprint")
        .arg("-sha1")
        .arg("-modulus")
        .arg("-text")
        .arg("-noout")
        .arg("-in")
        .arg(codesign0_path);

    let output = match cmd.output() {
        Ok(o) => o,
        Err(_) => {
            return Err("执行脚本失败".into());
        }
    };

    let err_str = String::from_utf8(output.stderr).unwrap();
    let output_str = String::from_utf8(output.stdout).unwrap();

    // println!("执行结果1{}", output_str);

    let lines: Vec<&str> = output_str.lines().take(2).collect();

    let sha1_tmp = lines[0].replace(":", "");

    let sha1 = match sha1_tmp.split("=").last() {
        Some(s) => s,
        None => "",
    };
    let modulus = match lines[1].split("=").last() {
        Some(s) => s,
        None => "",
    };

    //No such file or directory
    // println!("执行结果2{}", err_str);

    // let result = format!(
    //     "证书MD5指纹(签名MD5值)：{}\nModulus(公钥)：{}",
    //     sha1, modulus
    // );

    Ok((String::from(sha1), String::from(modulus)))
}

/**
 * 解析plist文件
 *
 * - 获取APP名称 CFBundleName
 * - 获取bundle id CFBundleIdentifier
 * - 获取APP图标 CFBundleIcons->CFBundlePrimaryIcon->CFBundleIconFiles
 */
fn parse_plist(plist_path: &str) -> Result<(String, String, String), String> {
    let app_path = Path::new(plist_path);

    let plist_path = app_path.join("Info.plist").as_path().display().to_string();

    let value = Value::from_file(plist_path).unwrap();
    let property = value.as_dictionary();
    let app_name = match property
        .and_then(|dict| dict.get("CFBundleName"))
        .and_then(|name| name.as_string())
    {
        Some(n) => n,
        None => "",
    };

    let bundle_id = match property
        .and_then(|dict| dict.get("CFBundleIdentifier"))
        .and_then(|name| name.as_string())
    {
        Some(n) => n,
        None => "",
    };

    let icon = match property
        .and_then(|dict| dict.get("CFBundleIcons"))
        .and_then(|name| name.as_dictionary())
        .and_then(|dict| dict.get("CFBundlePrimaryIcon"))
        .and_then(|arr| arr.as_dictionary())
        .and_then(|dict| dict.get("CFBundleIconFiles"))
        .and_then(|arr| arr.as_array())
        .and_then(|arr| arr.get(0))
        .and_then(|d| d.as_string())
    {
        Some(n) => n,
        None => "",
    };

    let mut icon_path = String::new();
    if let Ok(entries) = fs::read_dir(app_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let file_path = entry.path();
                if let Some(file_name) = file_path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        let filexxxx = file_name_str;

                        if file_path.is_file()
                            && String::from(filexxxx).contains(icon)
                            && file_path
                                .extension()
                                .map(|ext| ext == ".png" || ext == "png")
                                .unwrap_or(false)
                        {
                            icon_path = file_path.as_path().display().to_string();
                        }
                    }
                }
            }
        }
    }

    let icon_path = match get_normalized_png(&icon_path) {
        Ok(p) => p,
        Err(_) => String::from(""),
    };

    let mut file = File::open(icon_path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    let base64_string = general_purpose::STANDARD.encode(&buffer);

    Ok((
        String::from(app_name),
        String::from(bundle_id),
        base64_string,
    ))
}

/**
 * 恢复图片
 *
 * 执行```xcrun -sdk iphoneos pngcrush \ -q -revert-iphone-optimizations -d AppIcon60x60@2x.png```
 */
fn get_normalized_png(filename: &str) -> Result<String, String> {
    let mut cmd = Command::new("xcrun");

    let file = Path::new(filename);

    let parent = match file.parent() {
        Some(p) => p,
        None => {
            return Err("解析目录失败".into());
        }
    };

    cmd.current_dir(parent)
        .arg("-sdk")
        .arg("iphoneos")
        .arg("pngcrush")
        .arg("\\")
        .arg("-q")
        .arg("-revert-iphone-optimizations")
        .arg("-d")
        .arg(filename);

    let output = match cmd.output() {
        Ok(o) => o,
        Err(_) => {
            return Err("执行脚本失败".into());
        }
    };

    let err_str = String::from_utf8(output.stderr).unwrap();
    let output_str = String::from_utf8(output.stdout).unwrap();

    println!("执行结果1{}", output_str);
    println!("执行结果2{}", err_str);

    let file_name = file.file_name().unwrap().to_str().unwrap();

    let tmp = parent.join("-d").join(file_name);
    if tmp.exists() {
        Ok(tmp.as_path().display().to_string())
    } else {
        Err("反解失败".into())
    }
}
