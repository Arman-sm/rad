// Reference: https://web.archive.org/web/20141213140451/https://ccrma.stanford.edu/courses/422/projects/WaveFormat/

use std::mem::size_of;

use byteorder::{LittleEndian, BigEndian, WriteBytesExt};

// RIFF
const RIFF_MAGIC_CODE: i32 = 0x52494646;
const RIFF_CHUNK_SIZE: u32 = u32::MAX; 
const RIFF_FORMAT: i32 = 0x57415645;

// FMT Sub-Chunk
const SUB_CHUNK1_ID: i32 = 0x666d7420;

// DATA Sub-Chunk
const SUB_CHUNK2_ID: i32 = 0x64617461;
const SUB_CHUNK2_SIZE: u32 = u32::MAX - 44;

/// This function generates the metadata put at the first part of a wav file.
/// The size of the file is set to maximum as live cast can't have a fixed size.
pub fn gen_wav_header(sample_rate: u32, channels: u16) -> Vec<u8> {    
    let mut bin = Vec::new();
    bin.reserve_exact(44);

    // RIFF Chunk Descriptor
    bin.write_i32::<BigEndian>(RIFF_MAGIC_CODE).unwrap();
    bin.write_u32::<LittleEndian>(RIFF_CHUNK_SIZE).unwrap();
    bin.write_i32::<BigEndian>(RIFF_FORMAT).unwrap();

    // FMT Sub-Chunk
    let frame_size = channels * size_of::<i16>() as u16;
    let byte_rate = sample_rate * frame_size as u32;

    bin.write_i32::<BigEndian>(SUB_CHUNK1_ID).unwrap();
    bin.write_u32::<LittleEndian>(16).unwrap();
    bin.write_u16::<LittleEndian>(1).unwrap(); // Audio Format
    bin.write_u16::<LittleEndian>(channels).unwrap(); // Channels
    bin.write_u32::<LittleEndian>(sample_rate).unwrap(); // Sample rate
    bin.write_u32::<LittleEndian>(byte_rate).unwrap(); // Byte rate
    bin.write_i16::<LittleEndian>(frame_size as i16).unwrap(); // Block align
    bin.write_u16::<LittleEndian>(16).unwrap(); // Bits per sample
    // bin.write_u16::<LittleEndian>(0).unwrap(); // Extra param size

    // Data Sub-Chunk
    bin.write_i32::<BigEndian>(SUB_CHUNK2_ID).unwrap();
    bin.write_u32::<LittleEndian>(SUB_CHUNK2_SIZE).unwrap();

    bin
}