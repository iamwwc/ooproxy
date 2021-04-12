use std::ops::Range;
use std::str::from_utf8;
const EXT_SERVER_NAME: &[u8] = &[0, 0];
// len_range作为长度，获取长度之内的数据
// 0x01 0x02 0x03 0x04
// 0x01 表明长度为1
// 最后获得 0x02
fn slice_by_len_at_range(data: &[u8], len_range: Range<usize>) -> Result<&[u8], &'static str> {
    let len_in_bits = data
        .get(len_range.clone())
        .ok_or("no enough data length to decode{}")?;
    let mut actual_len = 0usize;
    for bit in len_in_bits {
        actual_len = actual_len << 8 | (*bit as usize)
    }
    data.get(len_range.end .. len_range.end +  actual_len).ok_or("error when get index")
}

// 移除 len_range.end 之前的数据
// 保留 len_range.end 之后的数据
// 常用来跳过 length + data 的组合
// 0x01 0x02 0x03 0x04
// 0x01 表明长度为1
// 最后获得 0x03 及之后数据
fn truncate_before(data: &[u8], len_range: Range<usize>) -> Result<&[u8], &'static str>{
    let len = slice_by_len_at_range(data, len_range.clone())?.len();
    Ok(&data[len_range.end + len ..])
}


pub struct TlsRecord<'a> {
    content_type: u8,
    // struct {
    //     uint8 major;
    //     uint8 minor;
    // } ProtocolVersion;
    major_version: u8,
    minor_version: u8,
    fragment: &'a [u8]
}

// 解析 TlsClientHello，我们当前只关心 server_name
// https://tools.ietf.org/html/rfc6066#section-3
pub struct TlsClientHello {
    server_name: Option<Box<str>>
}
pub fn parse_tls_record<'a>(data: &'a [u8]) -> Result<TlsRecord<'a>, &'static str> {
    let fragment = slice_by_len_at_range(&data, 3..5)?;
    Ok(TlsRecord {
        content_type: data[0],
        major_version: data[1],
        minor_version: data[2],
        fragment,
    })
}
pub fn parse_client_hello(data: &[u8]) -> Result<TlsClientHello, &'static str>{
    let TlsRecord {
        content_type,
        major_version,
        minor_version,
        fragment
    } = parse_tls_record(&data)?;
    if major_version != 3 {
        return Err("unknow tls version");
    }
    if content_type != 22 {
        return Err("not a handshake");
    }
    if fragment.get(0) != Some(&1) {
        return Err(" Handshake Type isn't a client hello");
    }
    // Handshake Protocol Client Hello Length is 3 bytes
    let client_hello_body = slice_by_len_at_range(&data, 1..4)?;
    // version: TLS 1.2 (0x0303)
    if client_hello_body.get(0) != Some(&0x03) {
        return Err("unsupported TLS version");
    }

    // Random 32bytes
    // Session ID Length 2 bytes
    // Session ID 
    // 34..35 Session ID Length
    let remaining = truncate_before(&data, 34..35)?;
    // Cipher Suites Length
    let remaining = truncate_before(&data, 0..2)?;
    // compression method
    let remaining = truncate_before(&data, 0..1)?;
    // extensions length
    let mut exts = slice_by_len_at_range(&data, 0..1)?;
    // extensions
    // type 2 bytes
    // length 2 bytes
    let mut server_name = None;
    while exts.len() > 4 {
        let ext_type = &exts[0..2];
        let ext_data = slice_by_len_at_range(&exts, 2..4)?;
        // 移除掉当前extension
        // 这样 exts 就以下一次extension开头
        exts = truncate_before(&exts, 2..4)?;
        if ext_type == EXT_SERVER_NAME {
            // server_name extension
            if ext_data[3] == 0x00 {
                let raw_name = slice_by_len_at_range(&ext_data, 3..5)?;
                let raw_name = from_utf8(&raw_name).map_err(|_| "error when parse from raw data")?;
                server_name = Some(String::from(raw_name).into_boxed_str());
            }
        }
    }
    Ok(TlsClientHello {
        server_name
    })
}

// struct {
//     ProtocolVersion client_version;
//     Random random;
//     SessionID session_id;
//     CipherSuite cipher_suites<2..2^16-2>;
//     CompressionMethod compression_methods<1..2^8-1>;
//     select (extensions_present) {
//         case false:
//             struct {};
//         case true:
//             Extension extensions<0..2^16-1>;
//     };
// } ClientHello;
#[test]
fn test_parse() {
    let data = [
        0x16, 0x03, 0x01, 0x00, 0xa1, 0x01, 0x00, 0x00, 0x9d, 0x03, 0x03, 0x52, 0x36, 0x2c, 0x10,
        0x12, 0xcf, 0x23, 0x62, 0x82, 0x56, 0xe7, 0x45, 0xe9, 0x03, 0xce, 0xa6, 0x96, 0xe9, 0xf6,
        0x2a, 0x60, 0xba, 0x0a, 0xe8, 0x31, 0x1d, 0x70, 0xde, 0xa5, 0xe4, 0x19, 0x49, 0x00, 0x00,
        0x04, 0xc0, 0x30, 0x00, 0xff, 0x02, 0x01, 0x00, 0x00, 0x6f, 0x00, 0x0b, 0x00, 0x04, 0x03,
        0x00, 0x01, 0x02, 0x00, 0x0a, 0x00, 0x34, 0x00, 0x32, 0x00, 0x0e, 0x00, 0x0d, 0x00, 0x19,
        0x00, 0x0b, 0x00, 0x0c, 0x00, 0x18, 0x00, 0x09, 0x00, 0x0a, 0x00, 0x16, 0x00, 0x17, 0x00,
        0x08, 0x00, 0x06, 0x00, 0x07, 0x00, 0x14, 0x00, 0x15, 0x00, 0x04, 0x00, 0x05, 0x00, 0x12,
        0x00, 0x13, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x0f, 0x00, 0x10, 0x00, 0x11, 0x00,
        0x23, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x22, 0x00, 0x20, 0x06, 0x01, 0x06, 0x02, 0x06, 0x03,
        0x05, 0x01, 0x05, 0x02, 0x05, 0x03, 0x04, 0x01, 0x04, 0x02, 0x04, 0x03, 0x03, 0x01, 0x03,
        0x02, 0x03, 0x03, 0x02, 0x01, 0x02, 0x02, 0x02, 0x03, 0x01, 0x01, 0x00, 0x0f, 0x00, 0x01,
        0x01,
    ];
    let TlsClientHello {
        server_name
    } = parse_client_hello(&data).unwrap();
}