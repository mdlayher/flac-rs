//! A pure Rust FLAC metadata parser, written as an experiment to learn more
//! about Rust.

extern crate byteorder;

use byteorder::{ByteOrder, BE, LE};
use std::io;
use std::io::prelude::*;
use std::str;

/// Specifies the type of metadata block found in a FLAC file.
#[derive(Debug)]
pub enum Block {
    StreamInfo(StreamInfo),
    Padding,
    Application,
    SeekTable,
    VorbisComment(VorbisComment),
    CueSheet,
    Picture,
    Reserved,
    Invalid,
}

/// Contains a FLAC file stream which can be parsed.
#[derive(Debug)]
pub struct Stream<'a, T: 'a + Read + Seek> {
    stream: &'a mut T,
}

impl<'a, T: Read + Seek> Stream<'a, T> {
    /// Creates a new Stream by accepting an input with traits Read and Seek.
    pub fn new(stream: &'a mut T) -> io::Result<Self> {
        let mut magic_buf = [0; 4];
        stream.read_exact(&mut magic_buf)?;

        let magic: [u8; 4] = [b'f', b'L', b'a', b'C'];

        if magic_buf != magic {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "incorrect FLAC magic number",
            ));
        }

        Ok(Stream { stream })
    }

    /// Produces a vector of tuples containing metadata headers and their
    /// associated metadata blocks.
    pub fn blocks(&mut self) -> io::Result<Vec<(Header, Block)>> {
        let mut blocks = Vec::new();

        // Each metadata header is 4 bytes.
        let mut meta_buf = [0; 4];
        loop {
            self.stream.read_exact(&mut meta_buf)?;
            let metadata = parse_header(meta_buf);

            // Block length indicates how much data we need to parse the next block.
            let mut block_buf = vec![0; metadata.block_length as usize];
            self.stream.read_exact(&mut block_buf)?;

            let block = match metadata.block_type {
                0 => Block::StreamInfo(parse_stream_info(&block_buf)?),
                1 => Block::Padding,
                2 => Block::Application,
                3 => Block::SeekTable,
                4 => Block::VorbisComment(parse_vorbis_comment(&block_buf)?),
                5 => Block::CueSheet,
                6 => Block::Picture,
                7...126 => Block::Reserved,
                _ => Block::Invalid,
            };

            // Are there any more blocks in this stream?
            if metadata.last_block {
                blocks.push((metadata, block));
                break;
            }

            blocks.push((metadata, block));
        }

        Ok(blocks)
    }
}

/// Contains the information found in the FLAC METADATA_BLOCK_HEADER structure.
#[derive(Debug)]
pub struct Header {
    pub last_block: bool,
    pub block_type: u8,
    pub block_length: u32,
}

fn parse_header(buf: [u8; 4]) -> Header {
    Header {
        last_block: (buf[0] >> 7) == 1,                       // 1 bit.
        block_type: buf[0] & 0x7f,                            // 7 bits.
        block_length: BE::read_u32(&buf[0..4]) & 0x00ff_ffff, // 24 bits.
    }
}

/// Contains the information found in the FLAC METADATA_BLOCK_STREAMINFO
/// structure.
#[derive(Debug)]
pub struct StreamInfo {
    pub minimum_block_size: u16,
    pub maximum_block_size: u16,
    pub minimum_frame_size: u32,
    pub maximum_frame_size: u32,
    pub sample_rate: u32,
    pub channels: u8,
    pub bits_per_sample: u8,
    pub total_samples: u64,
    pub md5_signature: [u8; 16],
}

fn parse_stream_info(buf: &[u8]) -> io::Result<StreamInfo> {
    if buf.len() != 34 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "incorrect size for FLAC stream info block",
        ));
    }

    let mut info = StreamInfo {
        minimum_block_size: BE::read_u16(&buf[0..2]),
        maximum_block_size: BE::read_u16(&buf[2..4]),
        minimum_frame_size: BE::read_u32(&buf[4..8]) >> 8,
        maximum_frame_size: BE::read_u32(&buf[7..11]) >> 8,
        sample_rate: BE::read_u32(&buf[10..14]) >> 12,
        channels: buf[12] & 0x0e,
        bits_per_sample: ((buf[12] & 0x01) | ((buf[13] & 0xf0) >> 4)) + 1,
        total_samples: (BE::read_u64(&buf[13..21]) & 0x0fff_ffff_ff00_0000) >> 24,
        md5_signature: [0 as u8; 16],
    };

    info.md5_signature.copy_from_slice(&buf[18..34]);

    Ok(info)
}

/// Contains the information found in the FLAC METADATA_BLOCK_VORBIS_COMMENT
/// structure.
#[derive(Debug)]
pub struct VorbisComment {
    pub vendor_string: String,
    pub user_comments: Vec<String>,
}

fn parse_vorbis_comment(buf: &[u8]) -> io::Result<VorbisComment> {
    // TODO(mdlayher): is there a better way to parse a slice?

    // Vorbis comments use little-endian integers:
    // https://www.xiph.org/vorbis/doc/v-comment.html.
    let vendor_length = LE::read_u32(&buf[0..4]);
    let vendor_string = str::from_utf8(&buf[4..4 + vendor_length as usize])
        .unwrap() // TODO: error conversion for io::Result.
        .to_string();

    let mut idx = 4 + vendor_length as usize;
    let user_comment_list_length = LE::read_u32(&buf[idx..idx + 4]);
    idx += 4;

    let mut user_comments = Vec::new();
    for _ in 0..user_comment_list_length {
        let comment_length = LE::read_u32(&buf[idx..idx + 4]);
        idx += 4;

        let comment = str::from_utf8(&buf[idx..idx+comment_length as usize])
        .unwrap() // TODO: error conversion for io::Result.
        .to_string();
        idx += comment_length as usize;

        user_comments.push(comment);
    }

    Ok(VorbisComment {
        vendor_string,
        user_comments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_number_ok() {
        let mut cursor = io::Cursor::new(vec![b'f', b'L', b'a', b'C']);
        let _ = Stream::new(&mut cursor).expect("expected valid FLAC magic number");
    }

    #[test]
    fn magic_number_bad() {
        let mut cursor = io::Cursor::new(vec![b'f', b'L', b'a', b'X']);
        let _ = Stream::new(&mut cursor).expect_err("expected invalid FLAC magic number");
    }
}
