extern crate byteorder;

use self::byteorder::{ByteOrder, BigEndian};
use std::vec::Vec;

enum Opcode {
    Cont,
    Text,
    Binary,
    NonControl(u8),
    Close,
    Ping,
    Pong,
    Control(u8)
}

fn unstrict_parse_frame(buf: Vec<u8>) {
    let start = 0;
    let byte = buf[start];
    let fin  = (byte >> 7) & 1;
    let rsv1 = (byte >> 6) & 1;
    let rsv2 = (byte >> 5) & 1;
    let rsv3 = (byte >> 4) & 1;
    let opcode =  byte & 0x0f;
    let opcode = match opcode {
        0x0 => Opcode::Cont,
        0x1 => Opcode::Text,
        0x2 => Opcode::Binary,
        0x3...0x7 => Opcode::NonControl(opcode),
        0x8 => Opcode::Close,
        0x9 => Opcode::Ping,
        0xa => Opcode::Pong,
        0xb...0xf => Opcode::Control(opcode),
        _   => panic!("logic flaw")
    };
    let start = start + 1;
    let byte = buf[start];
    let mask = (byte >> 7) & 1;
    let payload_len = byte & 0x7f;
    let start = start + 1;
    let (payloal_len, start) = match payload_len  {
        0 ... 125 => (payload_len as usize, start),
        126 => (BigEndian::read_u16(&buf[start..]) as usize, start + 4),
        127 => (BigEndian::read_u64(&buf[start..]) as usize, start + 8),
        _   => panic!("logic flaw")
    };
    // ensure payload is in 0 - 0x7FFFFFFFFFFFFFFF
    let mut masking_key = [0u8;4];
    let (masking_key, start) = {
        if mask == 1 {
            for i in 0..4 {
                masking_key[i] = buf[start + i]
            };
        }
        (masking_key, start + 4)
    };
    let (ext, start) = if rsv1 == 0 && rsv2 == 0 && rsv3 == 0 {
        (&[] as &[u8], start)
    } else {
        panic!("unknown extension")
    };
    let app = & buf[start..(start + payloal_len - ext.len())];
    let mut result = Vec::new();
    let range = (0..app.len());
    if mask == 1 {
        for i in range {
            result.push(app[i] ^ masking_key[i % 4]);
        }
    }
}

