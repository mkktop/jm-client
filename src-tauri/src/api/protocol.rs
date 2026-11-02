use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt, KeyInit};
use aes::Aes256;
use base64::engine::general_purpose;
use base64::Engine;
use md5::{Digest, Md5};

pub const DEFAULT_API_VERSION: &str = "2.1.12";
pub const APP_TOKEN_SECRET: &str = "185Hcomic3PAPP7R";
pub const CHAPTER_TOKEN_SECRET: &str = "18comicAPPContent";
pub const APP_DATA_SECRET: &str = "185Hcomic3PAPP7R";
pub const DOMAIN_SERVER_SECRET: &str = "diosfjckwpqpdfjkvnqQjsik";

pub const USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 9; V1938CT Build/PQ3A.190705.11211812; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/91.0.4472.114 Safari/537.36";

pub const API_DOMAINS: &[&str] = &[
    "www.cdnhjk.net",
    "www.cdngwc.cc",
    "www.cdngwc.net",
    "www.cdngwc.club",
    "www.cdnhth.net",
];

pub const DOMAIN_SERVERS: &[&str] = &[
    "https://rup4a04-c01.tos-ap-southeast-1.bytepluses.com/newsvr-2025.txt",
    "https://rup4a04-c02.tos-cn-hongkong.bytepluses.com/newsvr-2025.txt",
    "https://rup4a04-c03.tos-cn-beijing.bytepluses.com.cn/newsvr-2025.txt",
];

pub const IMAGE_HOSTS: &[&str] = &[
    "cdn-msp.jmapiproxy1.cc",
    "cdn-msp.jmapiproxy2.cc",
    "cdn-msp2.jmapiproxy2.cc",
    "cdn-msp3.jmapiproxy2.cc",
    "cdn-msp.jmapinodeudzn.net",
    "cdn-msp3.jmapinodeudzn.net",
];

pub fn md5_hex(value: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn token_and_param(ts: &str, version: &str, secret: &str) -> (String, String) {
    (
        md5_hex(&format!("{ts}{secret}")),
        format!("{ts},{version}"),
    )
}

fn aes_ecb_decrypt(ciphertext: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    if ciphertext.is_empty() || ciphertext.len() % 16 != 0 {
        return Err("密文长度不是 16 的倍数".to_string());
    }
    let cipher = Aes256::new_from_slice(key).map_err(|e| format!("AES key 无效: {e}"))?;
    let mut output = ciphertext.to_vec();
    for chunk in output.chunks_mut(16) {
        let mut block = GenericArray::clone_from_slice(chunk);
        cipher.decrypt_block(&mut block);
        chunk.copy_from_slice(&block);
    }
    Ok(output)
}

fn strip_pkcs7(data: &mut Vec<u8>) -> Result<(), String> {
    let Some(last) = data.last().copied() else {
        return Err("解密结果为空".to_string());
    };
    let padding = last as usize;
    if padding == 0 || padding > 16 || padding > data.len() {
        return Err("PKCS#7 padding 无效".to_string());
    }
    data.truncate(data.len() - padding);
    Ok(())
}

pub fn decrypt_api_data(data: &str, ts: &str) -> Result<String, String> {
    let ciphertext = general_purpose::STANDARD
        .decode(data)
        .map_err(|e| format!("base64 解码失败: {e}"))?;
    let key = md5_hex(&format!("{ts}{APP_DATA_SECRET}"));
    let mut plain = aes_ecb_decrypt(&ciphertext, key.as_bytes())?;
    strip_pkcs7(&mut plain)?;
    String::from_utf8(plain).map_err(|e| format!("解密结果不是 UTF-8: {e}"))
}

pub fn decrypt_domain_server(data: &str) -> Result<String, String> {
    let cleaned: String = data
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii())
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let ciphertext = general_purpose::STANDARD
        .decode(cleaned)
        .map_err(|e| format!("域名服务器 base64 解码失败: {e}"))?;
    let key = md5_hex(DOMAIN_SERVER_SECRET);
    let mut plain = aes_ecb_decrypt(&ciphertext, key.as_bytes())?;
    strip_pkcs7(&mut plain)?;
    String::from_utf8(plain).map_err(|e| format!("域名服务器解密结果不是 UTF-8: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_is_stable() {
        assert_eq!(
            md5_hex(&format!("1793601586{APP_TOKEN_SECRET}")),
            md5_hex("1793601586185Hcomic3PAPP7R")
        );
    }

    #[test]
    fn tokenparam_contains_version() {
        let (token, param) = token_and_param("123", "2.1.12", APP_TOKEN_SECRET);
        assert_eq!(token.len(), 32);
        assert_eq!(param, "123,2.1.12");
    }
}
