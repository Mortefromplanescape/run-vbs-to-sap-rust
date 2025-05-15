use anyhow::Context;
use encoding_rs::UTF_16LE;
use chardetng::EncodingDetector;

pub fn execute_script(temp_path: &std::path::Path) -> anyhow::Result<String> {    
    let output = std::process::Command::new("cscript.exe")
        .args(&["//Nologo", &temp_path.to_string_lossy()])
        .output()
        .context("Ошибка выполнения скрипта")?;

    let stdout = decode_output(&output.stdout);
    let stderr = decode_output(&output.stderr);

    let mut result = String::new();
    if !stdout.is_empty() {
        result.push_str(&format!("[Вывод]\n{}\n", clean_output(stdout)));
    }
    if !stderr.is_empty() {
        result.push_str(&format!("[Ошибки]\n{}\n", clean_output(stderr)));
    }
    
    if output.status.success() {
        Ok(result.trim().to_string())
    } else {
        anyhow::bail!("Код ошибки: {}\n{}", output.status, result.trim())
    }
}

fn decode_output(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return UTF_16LE.decode(&bytes[2..]).0.into_owned();
    }

    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let encoding = detector.guess(None, true);
    encoding.decode(bytes).0.into_owned()
}

fn clean_output(s: String) -> String {
    s.replace('\0', "")
        .trim()
        .replace("\r\n", "\n")
        .to_string()
}