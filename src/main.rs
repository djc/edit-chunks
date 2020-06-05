use std::cmp::min;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::iter;
use std::num::ParseIntError;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use structopt::StructOpt;

fn main() {
    let spec = Command::from_args();
    match spec {
        Command::Split { path, ranges } => split(path, ranges),
        Command::Combine { spec } => combine(spec),
    }
}

#[derive(StructOpt, Debug, Serialize)]
#[structopt(
    name = "edit-chunks",
    about = "Split out chunks of a large file for editing,
then put them back together again."
)]
enum Command {
    #[structopt(name = "split", about = "split a file")]
    Split { path: String, ranges: Vec<Range> },
    #[structopt(name = "combine", about = "combine a previously split file again")]
    Combine { spec: String },
}

fn split(path: String, ranges: Vec<Range>) {
    let mut spec_file_name = path.clone();
    spec_file_name.push_str(".spec");
    eprintln!("writing specification to {:?}...", &spec_file_name);
    let mut spec_file = File::create(spec_file_name).unwrap();
    let spec = Spec { path, ranges };
    serde_json::to_writer(&mut spec_file, &spec).unwrap();

    let mut in_file = File::open(&spec.path).unwrap();
    let mut part_file_name = String::new();
    let mut buf = Vec::new();
    for (i, &Range { start, end }) in spec.ranges.iter().enumerate() {
        set_file_name(&mut part_file_name, &spec.path, i);
        resize_buffer(&mut buf, (end - start) as usize);
        eprintln!(
            "writing bytes {}-{} to file {:?}...",
            start, end, part_file_name
        );
        let mut part_file = File::create(&part_file_name).unwrap();
        in_file.seek(SeekFrom::Start(start)).unwrap();
        in_file.read(&mut buf).unwrap();
        part_file.write(&buf).unwrap();
    }

    eprintln!("done");
}

fn combine(spec_file_name: String) {
    let spec_file = File::open(&spec_file_name).unwrap();
    let spec: Spec = serde_json::from_reader(spec_file).unwrap();

    let mut new_ranges = Vec::new();
    let mut last = 0;
    for (i, &Range { start, end }) in spec.ranges.iter().enumerate() {
        if start > last {
            new_ranges.push(Provenance::Old(Range {
                start: last,
                end: start,
            }));
        }
        new_ranges.push(Provenance::New(i, Range { start, end }));
        last = end;
    }

    let mut in_file = File::open(&spec.path).unwrap();
    let file_len = in_file.metadata().unwrap().len();
    new_ranges.push(Provenance::Old(Range {
        start: last,
        end: file_len,
    }));
    let mut out_file_name = spec.path.clone();
    out_file_name.push_str(".new");

    let mut part_file_name = String::new();
    let mut out_file = File::create(&out_file_name).unwrap();
    let mut buf = Vec::new();
    for prov in new_ranges.iter() {
        match *prov {
            Provenance::Old(Range { start, end }) => {
                in_file.seek(SeekFrom::Start(start)).unwrap();
                let mut remaining = (end - start) as usize;
                let chunks = remaining / CHUNK_SIZE;
                let mut idx = 0;
                while remaining > 0 {
                    let next_chunk = min(remaining, CHUNK_SIZE);
                    resize_buffer(&mut buf, next_chunk);
                    in_file.read(&mut buf).unwrap();
                    out_file.write(&buf).unwrap();
                    eprintln!(
                        "copying {} bytes from {} (chunk {}/{})...",
                        next_chunk, &spec.path, idx, chunks
                    );
                    remaining -= next_chunk;
                    idx += 1;
                }
            }
            Provenance::New(idx, Range { start, end }) => {
                set_file_name(&mut part_file_name, &spec.path, idx);
                let mut part_file = File::open(&part_file_name).unwrap();
                buf.truncate(0);
                part_file.read_to_end(&mut buf).unwrap();

                let new_len = buf.len();
                let old_len = (end - start) as usize;
                let diff_str = if old_len == new_len {
                    "same size".to_owned()
                } else if old_len < new_len {
                    format!("+{} bytes", new_len - old_len)
                } else {
                    format!("-{} bytes", old_len - new_len)
                };

                eprintln!(
                    "copying {} bytes from {} ({})...",
                    new_len, &part_file_name, diff_str
                );
                out_file.write(&buf).unwrap();
            }
        }
    }

    eprintln!("done");
}

fn resize_buffer(buf: &mut Vec<u8>, new_size: usize) {
    let buf_len = buf.len();
    if buf_len < new_size {
        buf.extend(iter::repeat(0).take(new_size - buf_len));
    } else if buf_len > new_size {
        buf.truncate(new_size);
    }
}

fn set_file_name(s: &mut String, orig: &str, idx: usize) {
    s.clear();
    s.push_str(orig);
    s.push_str(&format!(".part.{}", idx));
}

#[derive(Debug)]
enum Provenance {
    Old(Range),
    New(usize, Range),
}

const CHUNK_SIZE: usize = 16 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
struct Spec {
    pub path: String,
    pub ranges: Vec<Range>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Range {
    start: u64,
    end: u64,
}

impl FromStr for Range {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let vals: Vec<&str> = s.split("-").collect();
        Ok(Range {
            start: vals[0].parse::<u64>()?,
            end: vals[1].parse::<u64>()?,
        })
    }
}
