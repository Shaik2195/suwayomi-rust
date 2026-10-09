use crate::types::{ExtensionListing, ExtensionRepo};
use suwayomi_core::error::{Result, SuwayomiError};
use reqwest;
use std::io::Read;
use flate2::read::GzDecoder;

pub struct ExtensionRegistry {
    pub repos: Vec<ExtensionRepo>,
}

fn decode_varint(bytes: &[u8], pos: &mut usize) -> Option<u64> {
    let mut result = 0;
    let mut shift = 0;
    while *pos < bytes.len() {
        let b = bytes[*pos];
        *pos += 1;
        result |= ((b & 0x7F) as u64) << shift;
        if b & 0x80 == 0 {
            return Some(result);
        }
        shift += 7;
        if shift >= 64 {
            return None; // too long
        }
    }
    None
}

fn read_string(bytes: &[u8], pos: &mut usize) -> Option<String> {
    let len = decode_varint(bytes, pos)? as usize;
    if *pos + len <= bytes.len() {
        let s = std::str::from_utf8(&bytes[*pos..*pos + len]).ok()?.to_string();
        *pos += len;
        Some(s)
    } else {
        None
    }
}

fn skip_field(bytes: &[u8], pos: &mut usize, wire_type: u8) -> Option<()> {
    match wire_type {
        0 => { // varint
            decode_varint(bytes, pos)?;
            Some(())
        },
        1 => { // 64-bit
            if *pos + 8 <= bytes.len() {
                *pos += 8;
                Some(())
            } else {
                None
            }
        },
        2 => { // length-delimited
            let len = decode_varint(bytes, pos)? as usize;
            if *pos + len <= bytes.len() {
                *pos += len;
                Some(())
            } else {
                None
            }
        },
        5 => { // 32-bit
            if *pos + 4 <= bytes.len() {
                *pos += 4;
                Some(())
            } else {
                None
            }
        },
        _ => None, // unsupported or invalid wire type
    }
}

fn parse_protobuf(bytes: &[u8]) -> Option<Vec<ExtensionListing>> {
    let mut listings = Vec::new();
    let mut pos = 0;
    
    while pos < bytes.len() {
        let tag = decode_varint(bytes, &mut pos)?;
        let field_num = tag >> 3;
        let wire_type = (tag & 0x7) as u8;
        
        if field_num == 101 && wire_type == 2 {
            // Extension message (length-delimited)
            let len = decode_varint(bytes, &mut pos)? as usize;
            let end = pos + len;
            if end > bytes.len() {
                return None;
            }
            
            // To support both the old test structure (where 101 is the Extension) 
            // and the Keiyoushi structure (where 101 contains a repeated field 1 of Extensions)
            // we will parse the fields into variables. If we encounter field 1 as a submessage,
            // we parse it as a nested extension.
            
            let mut name = String::new();
            let mut pkg_name = String::new();
            let mut version_name = String::new();
            let mut version_code = 0;
            let mut is_nsfw = false;
            let mut apk_url = String::new();
            let mut icon_url = String::new();
            let mut lang = String::new();
            let mut is_direct = false;
            
            while pos < end {
                let inner_tag = decode_varint(bytes, &mut pos)?;
                let inner_field = inner_tag >> 3;
                let inner_wire_type = (inner_tag & 0x7) as u8;
                
                if inner_field == 1 && inner_wire_type == 2 {
                    // This could be Keiyoushi nested extension OR the direct extension's name (which is also field 1, wire 2, string)
                    // If it is a string, reading it as a string works. But if it's a submessage containing an extension, parsing it as string fails.
                    // We can check if it parses as string cleanly, or we can just assume Keiyoushi format wraps it in field 1 submessage.
                    // Let's use a nested loop for Keiyoushi.
                    let sub_len = decode_varint(bytes, &mut pos)? as usize;
                    let sub_end = pos + sub_len;
                    
                    // Look inside to see if it's a valid string or nested message.
                    // If we assume it's nested Keiyoushi Extension:
                    let mut ext_name = String::new();
                    let mut ext_pkg = String::new();
                    let mut ext_vname = String::new();
                    let mut ext_vcode = 0;
                    let mut ext_nsfw = false;
                    let mut ext_apk = String::new();
                    let mut ext_icon = String::new();
                    let mut ext_lang = String::new();
                    
                    let mut is_nested = false;
                    
                    // We try to parse it as Keiyoushi nested extension. If it fails, we fall back to reading it as string.
                    let backup_pos = pos;
                    while pos < sub_end {
                        if let Some(f_tag) = decode_varint(bytes, &mut pos) {
                            let f_field = f_tag >> 3;
                            let f_wire = (f_tag & 0x7) as u8;
                            match f_field {
                                1 => {
                                    if let Some(s) = read_string(bytes, &mut pos) {
                                        ext_name = s;
                                        is_nested = true;
                                    } else { break; }
                                },
                                2 => {
                                    if let Some(s) = read_string(bytes, &mut pos) {
                                        ext_pkg = s;
                                        is_nested = true;
                                    } else { break; }
                                },
                                3 => {
                                    if let Some(s_len) = decode_varint(bytes, &mut pos) {
                                        let s_end = pos + s_len as usize;
                                        while pos < s_end {
                                            if let Some(s_tag) = decode_varint(bytes, &mut pos) {
                                                match s_tag >> 3 {
                                                    1 => if let Some(s) = read_string(bytes, &mut pos) { ext_apk = s; },
                                                    2 => if let Some(s) = read_string(bytes, &mut pos) { ext_icon = s; },
                                                    _ => { skip_field(bytes, &mut pos, (s_tag & 0x7) as u8); }
                                                }
                                            }
                                        }
                                    }
                                },
                                5 => {
                                    if let Some(v) = decode_varint(bytes, &mut pos) {
                                        ext_vcode = v as i64;
                                    }
                                },
                                6 => {
                                    if let Some(s) = read_string(bytes, &mut pos) {
                                        ext_vname = s;
                                    }
                                },
                                7 => {
                                    if let Some(v) = decode_varint(bytes, &mut pos) {
                                        ext_nsfw = v != 0;
                                    }
                                },
                                8 => {
                                    if let Some(s_len) = decode_varint(bytes, &mut pos) {
                                        let s_end = pos + s_len as usize;
                                        while pos < s_end {
                                            if let Some(s_tag) = decode_varint(bytes, &mut pos) {
                                                if (s_tag >> 3) == 3 {
                                                    if let Some(s) = read_string(bytes, &mut pos) {
                                                        if ext_lang.is_empty() { ext_lang = s; }
                                                    }
                                                } else {
                                                    skip_field(bytes, &mut pos, (s_tag & 0x7) as u8);
                                                }
                                            }
                                        }
                                    }
                                },
                                _ => { skip_field(bytes, &mut pos, f_wire); }
                            }
                        } else {
                            break;
                        }
                    }
                    
                    if is_nested && !ext_pkg.is_empty() {
                        if ext_lang.is_empty() {
                            let parts: Vec<&str> = ext_pkg.split('.').collect();
                            ext_lang = if parts.len() > 4 { parts[4].to_string() } else { "all".to_string() };
                        }
                        listings.push(ExtensionListing {
                            pkg_name: ext_pkg,
                            name: ext_name,
                            version_name: ext_vname,
                            version_code: ext_vcode,
                            lang: ext_lang,
                            is_nsfw: ext_nsfw,
                            apk_url: ext_apk,
                            icon_url: ext_icon,
                        });
                    } else {
                        // It was just a string (name field of direct extension)
                        pos = backup_pos;
                        if let Ok(s) = String::from_utf8(bytes[pos..sub_end].to_vec()) {
                            name = s;
                            is_direct = true;
                        }
                        pos = sub_end;
                    }
                } else {
                    // Fallback to direct parsing for tests/old format
                    is_direct = true;
                    match inner_field {
                        1 => {
                            name = read_string(bytes, &mut pos)?;
                        },
                        2 => {
                            pkg_name = read_string(bytes, &mut pos)?;
                        },
                        3 => {
                            let sub_len = decode_varint(bytes, &mut pos)? as usize;
                            let sub_end = pos + sub_len;
                            while pos < sub_end {
                                let sub_tag = decode_varint(bytes, &mut pos)?;
                                let sub_field = sub_tag >> 3;
                                let sub_wire_type = (sub_tag & 0x7) as u8;
                                match sub_field {
                                    1 => apk_url = read_string(bytes, &mut pos)?,
                                    2 => icon_url = read_string(bytes, &mut pos)?,
                                    _ => skip_field(bytes, &mut pos, sub_wire_type)?,
                                }
                            }
                        },
                        5 => {
                            version_code = decode_varint(bytes, &mut pos)? as i64;
                        },
                        6 => {
                            version_name = read_string(bytes, &mut pos)?;
                        },
                        7 => {
                            is_nsfw = decode_varint(bytes, &mut pos)? != 0;
                        },
                        8 => {
                            let sub_len = decode_varint(bytes, &mut pos)? as usize;
                            let sub_end = pos + sub_len;
                            while pos < sub_end {
                                let sub_tag = decode_varint(bytes, &mut pos)?;
                                let sub_field = sub_tag >> 3;
                                let sub_wire_type = (sub_tag & 0x7) as u8;
                                match sub_field {
                                    3 => {
                                        if lang.is_empty() {
                                            lang = read_string(bytes, &mut pos)?;
                                        } else {
                                            skip_field(bytes, &mut pos, sub_wire_type)?;
                                        }
                                    },
                                    _ => skip_field(bytes, &mut pos, sub_wire_type)?,
                                }
                            }
                        },
                        _ => skip_field(bytes, &mut pos, inner_wire_type)?,
                    }
                }
            }
            
            if is_direct && !pkg_name.is_empty() {
                if lang.is_empty() {
                    let parts: Vec<&str> = pkg_name.split('.').collect();
                    lang = if parts.len() > 4 { parts[4].to_string() } else { "all".to_string() };
                }
                
                listings.push(ExtensionListing {
                    pkg_name, name, version_name, version_code, lang, is_nsfw, apk_url, icon_url,
                });
            }
        } else {
            skip_field(bytes, &mut pos, wire_type)?;
        }
    }
    
    Some(listings)
}

impl ExtensionRegistry {
    pub fn new(repos: Vec<ExtensionRepo>) -> Self {
        Self { repos }
    }

    pub async fn fetch_repo_index(repo: &ExtensionRepo) -> Result<Vec<ExtensionListing>> {
        let response = reqwest::get(&repo.url)
            .await
            .map_err(|e| SuwayomiError::Network(e.to_string()))?;
            
        let bytes = response.bytes().await.map_err(|e| SuwayomiError::Network(e.to_string()))?;
        
        let mut decompressed = Vec::new();
        let mut is_protobuf = repo.url.ends_with(".pb");
        
        if bytes.starts_with(&[0x1f, 0x8b]) {
            let mut decoder = GzDecoder::new(&bytes[..]);
            decoder.read_to_end(&mut decompressed).map_err(|e| SuwayomiError::Parse(format!("Gzip decompress failed: {}", e)))?;
            is_protobuf = true; // Keiyoushi gzipped file is protobuf
        } else {
            decompressed = bytes.to_vec();
        }

        if is_protobuf {
            if let Some(listings) = parse_protobuf(&decompressed) {
                return Ok(listings);
            }
            return Err(SuwayomiError::Parse("Failed to parse protobuf index".to_string()));
        }

        // Fallback to JSON parsing
        let listings = serde_json::from_slice::<Vec<ExtensionListing>>(&decompressed)
            .map_err(|e| SuwayomiError::Parse(e.to_string()))?;
            
        Ok(listings)
    }

    pub async fn get_available_extensions(&self) -> Result<Vec<ExtensionListing>> {
        let mut all_listings = Vec::new();
        
        for repo in &self.repos {
            match Self::fetch_repo_index(repo).await {
                Ok(listings) => all_listings.extend(listings),
                Err(e) => {
                    // Log error but continue with other repos
                    eprintln!("Failed to fetch repo index from {}: {}", repo.url, e);
                }
            }
        }
        
        Ok(all_listings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    #[tokio::test]
    async fn test_fetch_repo_index() {
        let server = MockServer::start();
        
        let _mock = server.mock(|when, then| {
            when.method(GET).path("/index.min.json");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"[
                    {
                        "pkg_name": "eu.kanade.tachiyomi.extension.en.mangadex",
                        "name": "MangaDex",
                        "version_name": "1.2.3",
                        "version_code": 12,
                        "lang": "en",
                        "is_nsfw": false,
                        "apk_url": "/apk/mangadex.apk",
                        "icon_url": "/icon/mangadex.png"
                    }
                ]"#);
        });
        
        let repo = ExtensionRepo {
            name: "Test Repo".to_string(),
            url: server.url("/index.min.json"),
        };
        
        let listings = ExtensionRegistry::fetch_repo_index(&repo).await;
        
        assert!(listings.is_ok());
        let listings = listings.unwrap();
        
        assert_eq!(listings.len(), 1);
        assert_eq!(listings[0].name, "MangaDex");
        assert_eq!(listings[0].version_code, 12);
    }

    // Helper to write varint
    fn write_varint(val: u64, buf: &mut Vec<u8>) {
        let mut v = val;
        loop {
            let mut b = (v & 0x7F) as u8;
            v >>= 7;
            if v != 0 {
                b |= 0x80;
            }
            buf.push(b);
            if v == 0 {
                break;
            }
        }
    }

    // Helper to write length-delimited string
    fn write_string(tag: u64, val: &str, buf: &mut Vec<u8>) {
        write_varint((tag << 3) | 2, buf);
        let bytes = val.as_bytes();
        write_varint(bytes.len() as u64, buf);
        buf.extend_from_slice(bytes);
    }

    #[tokio::test]
    async fn test_fetch_repo_index_protobuf() {
        let mut pb_bytes = Vec::new();

        let mut ext_msg = Vec::new();
        write_string(1, "MockExt", &mut ext_msg);
        write_string(2, "eu.kanade.tachiyomi.extension.en.mock", &mut ext_msg);

        let mut sub_msg = Vec::new();
        write_string(1, "/mock.apk", &mut sub_msg);
        write_string(2, "/mock.png", &mut sub_msg);
        write_varint((3 << 3) | 2, &mut ext_msg);
        write_varint(sub_msg.len() as u64, &mut ext_msg);
        ext_msg.extend_from_slice(&sub_msg);

        write_varint((5 << 3) | 0, &mut ext_msg);
        write_varint(42, &mut ext_msg);
        write_string(6, "1.2.4", &mut ext_msg);
        write_varint((7 << 3) | 0, &mut ext_msg);
        write_varint(1, &mut ext_msg); 

        let mut source_msg = Vec::new();
        write_string(3, "fr", &mut source_msg);
        write_varint((8 << 3) | 2, &mut ext_msg);
        write_varint(source_msg.len() as u64, &mut ext_msg);
        ext_msg.extend_from_slice(&source_msg);

        write_varint((101 << 3) | 2, &mut pb_bytes);
        write_varint(ext_msg.len() as u64, &mut pb_bytes);
        pb_bytes.extend_from_slice(&ext_msg);

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&pb_bytes).unwrap();
        let gzipped = encoder.finish().unwrap();

        let server = MockServer::start();
        let _mock = server.mock(|when, then| {
            when.method(GET).path("/index.pb");
            then.status(200).body(gzipped);
        });
        
        let repo = ExtensionRepo { name: "Test PB Repo".to_string(), url: server.url("/index.pb") };
        let listings = ExtensionRegistry::fetch_repo_index(&repo).await.unwrap();
        
        assert_eq!(listings.len(), 1);
        assert_eq!(listings[0].name, "MockExt");
    }
}
