extern crate byteorder;

use self::byteorder::{ByteOrder, BigEndian};
use std::vec::Vec;

#[derive(Debug)]
pub enum Opcode {
    Cont,
    Text,
    Binary,
    NonControl(u8),
    Close,
    Ping,
    Pong,
    Control(u8)
}

fn byte_to_opcode(byte :u8) -> Opcode {
    match byte & 0x0f {
        0x0 => Opcode::Cont,
        0x1 => Opcode::Text,
        0x2 => Opcode::Binary,
        opcode @ 0x3...0x7 => Opcode::NonControl(opcode),
        0x8 => Opcode::Close,
        0x9 => Opcode::Ping,
        0xa => Opcode::Pong,
        opcode @ 0xb...0xf => Opcode::Control(opcode),
        _   => panic!("logic flaw")
    }
}

fn opcode_to_byte(opcode: Opcode) -> u8 {
    match opcode {
        Opcode::Cont               => 0x0,
        Opcode::Text               => 0x1,
        Opcode::Binary             => 0x2,
        Opcode::NonControl(opcode) => opcode,
        Opcode::Close              => 0x8,
        Opcode::Ping               => 0x9,
        Opcode::Pong               => 0xa ,
        Opcode::Control(opcode)    => opcode,
    }    
}

fn len_to_vec(len:usize) -> Result<Vec<u8>, String> {
    let mut res = Vec::new();
    match len {
        0...  0xfd => res.push(len as u8),
        0xfe...0xffff => {
            let mut buf = [0; 2];
            BigEndian::write_u16(&mut buf, len as u16);
            res.push(126u8);
            for b in buf.iter() {
                res.push(*b)
            }
        },
        0x10000...0x7fffffffffffffff => {
            let mut buf = [0; 8];
            BigEndian::write_u64(&mut buf, len as u64);
            res.push(127u8);
            for b in buf.iter() {
                res.push(*b)
            }
        }
        _ => return Err("Too long".to_string())
    }
    Ok(res)
}

pub fn parse_frame(buf: &[u8]) -> Result<(Opcode, Vec<u8>), String> {
    let start = 0;
    let byte = buf[start];
    let fin  = (byte >> 7) & 1;
    let rsv1 = (byte >> 6) & 1;
    let rsv2 = (byte >> 5) & 1;
    let rsv3 = (byte >> 4) & 1;
    let opcode = byte_to_opcode(byte & 0x0f);
    let start = start + 1;
    let byte = buf[start];
    let mask = (byte >> 7) & 1;
    let start = start + 1;
    let (payload_len, start) = match byte & 0x7f  {
        payload_len @ 0 ... 125 => (payload_len as usize, start),
        126 => (BigEndian::read_u16(&buf[start..]) as usize, start + 4),
        127 => (BigEndian::read_u64(&buf[start..]) as usize, start + 8),
        _   => panic!("logic flaw")
    };
    if  0x7FFFFFFFFFFFFFFF < payload_len {
        return Err("".to_string());
    }
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
        return Err("Unknown ext format".to_string())
    };
    let app = & buf[start..(start + payload_len - ext.len())];
    let mut result = Vec::new();
    if mask == 1 {
        for i in 0..app.len() {
            result.push(app[i] ^ masking_key[i % 4]);
        }
    }
    Ok((opcode, result))
}

pub fn pack_message(msg: &[u8], mask: Option<[u8;4]>) -> Result<Vec<u8>, String> {
    let mut frame = Vec::new();
    let len = msg.len();
    frame.push(0x8<<4 | opcode_to_byte(Opcode::Text));
    for b in len_to_vec(len).unwrap() {
        frame.push(b);
    }
    match mask {
        Some(m) => {
            frame[1] |=  0x80;
            for i in 0..len {
                frame.push(msg[i] ^ m[i % 4])
            }
        },
        None    => {
            for &b in msg {
                frame.push(b);
            }

        }
    }
    Ok(frame)
}
