// Copyright (C) 2022 amaanq

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use lzham;
use lzma;
use std::fmt::Write;
use std::fs;

fn main() {
    let mut compressor = new(String::from("helpshift.csv"));
    let data = compressor.decompress();
    let data = String::from_utf8(data).unwrap();
    println!("{}", data);
}

// Credit: https://github.com/jeanbmar/sc-compression/blob/master/src/sc-compression.js
#[derive(Debug)]
pub enum Signature {
    NONE,
    LZMA, // starts with 5D 00 00 04
    SC,   // starts with SC
    SCLZ, // starts with SC and contains SCLZ
    SIG,  // starts with Sig:
}
pub struct ScCompression {
    buffer: Vec<u8>,
}

pub fn new(fp: String) -> ScCompression {
    ScCompression {
        buffer: fs::read(fp).expect("Something went wrong reading the file"),
    }
}

pub fn new_from_buffer(buffer: Vec<u8>) -> ScCompression {
    ScCompression { buffer: (buffer) }
}

impl ScCompression {
    //‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾
    // Compression Section
    //_______________________________________________________________________

    pub fn compress(&mut self, sig: Signature) -> Vec<u8> {
        match sig {
            Signature::NONE => self.buffer.clone(),
            Signature::LZMA => self.compress_lzma(),
            Signature::SC => self.compress_sc(),
            Signature::SCLZ => self.compress_sclz(),
            Signature::SIG => self.compress_sig(),
        }
    }

    fn compress_lzma(&mut self) -> Vec<u8> {
        let mut compressed = lzma::compress(&self.buffer, 6).unwrap();
        compressed.drain(5..9);
        compressed
    }

    fn compress_sc(&mut self) -> Vec<u8> {
        let mut lzma_compressed = self.compress(Signature::LZMA);
        let mut compressed_with_header = vec![0; 26];
        compressed_with_header.append(&mut lzma_compressed);
        compressed_with_header
    }

    fn compress_sclz(&mut self) -> Vec<u8> {
        let mut lzham_compressed = Vec::new();
        let usize = self.buffer.len();
        let status = lzham::compress(&mut &self.buffer[..], &mut lzham_compressed);
        if !status.is_success() {
            panic!("{:?}", status);
        }
        let mut compressed_with_header = vec![0; 31];
        compressed_with_header.append(&mut usize.to_be_bytes().to_vec());
        compressed_with_header.append(&mut lzham_compressed);
        compressed_with_header
    }

    fn compress_sig(&mut self) -> Vec<u8> {
        let mut lzma_compressed = self.compress(Signature::LZMA);
        let mut compressed_with_header = String::from("Sig:").as_bytes().to_vec();
        compressed_with_header.append(vec![0; 64].to_vec().as_mut());
        compressed_with_header.append(&mut lzma_compressed);
        compressed_with_header
    }

    //‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾
    // Decompression Section
    //_______________________________________________________________________

    pub fn decompress(&mut self) -> Vec<u8> {
        let sig = self.read_signature();
        let decompressed = match sig {
            Signature::NONE => self.buffer.clone(),
            Signature::LZMA => self.decompress_lzma(),
            Signature::SC => self.decompress_sc(),
            Signature::SCLZ => self.decompress_sclz(),
            Signature::SIG => self.decompress_sig(),
        };
        decompressed.to_vec()
    }

    fn decompress_lzma(&mut self) -> Vec<u8> {
        let uncompressed_size = i32::from_le_bytes(self.buffer[5..9].try_into().unwrap());
        if uncompressed_size == -1 {
            self.buffer.append(&mut vec![0xFF; 4]);
            self.buffer[9..].rotate_right(4);
        } else {
            self.buffer.append(&mut vec![0x00; 4]);
            self.buffer[9..].rotate_right(4);
        }
        let data = lzma::decompress(&self.buffer).unwrap();
        return data;
    }

    fn decompress_sc(&mut self) -> Vec<u8> {
        self.buffer = self.buffer[26..].to_vec();
        self.decompress_lzma()
    }

    fn decompress_sclz(&mut self) -> Vec<u8> {
        let mut decompressed = Vec::new();
        let usize = usize::from_le_bytes(self.buffer[31..35].try_into().unwrap());
        let status = lzham::decompress(&mut &self.buffer[35..], &mut decompressed, usize);
        if !status.is_success() {
            panic!("{:?}", status);
        }
        decompressed
    }

    fn decompress_sig(&mut self) -> Vec<u8> {
        self.buffer = self.buffer[68..].to_vec();
        self.decompress_lzma()
    }

    fn read_signature(&self) -> Signature {
        if encode_hex(&self.buffer[..3]).to_lowercase() == "5d0000" {
            return Signature::LZMA;
        } else if vec8_to_lower_str(&self.buffer[..2].to_vec()) == "sc" {
            if self.buffer.len() > 30 && vec8_to_lower_str(&self.buffer[26..30].to_vec()) == "sclz"
            {
                return Signature::SCLZ;
            }
            return Signature::SC;
        } else if vec8_to_lower_str(&self.buffer[..4].to_vec()) == "sig:" {
            return Signature::SIG;
        }
        Signature::NONE
    }
}

fn vec8_to_lower_str(vec: &Vec<u8>) -> String {
    String::from_utf8(vec.to_vec())
        .expect("bad bytes")
        .to_lowercase()
}

pub fn encode_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        write!(&mut s, "{:02x}", b).unwrap();
    }
    s
}
