use std::{
    cell::RefCell,
    io::{Read, Seek, SeekFrom, Write},
};

thread_local! {
    static IO_TABLE: RefCell<IoTable> = RefCell::new(IoTable::new());
}

pub trait ReadSeek: Read + Seek {}

impl<T: Read + Seek> ReadSeek for T {}

pub enum IoTableEntry {
    Read(Box<dyn ReadSeek>),
    Write(Box<dyn Write>),
    Closed,
}

pub struct IoTable(Vec<IoTableEntry>);

impl IoTable {
    pub fn new() -> Self {
        IoTable(Vec::with_capacity(3))
    }

    fn push_entry(&mut self, entry: IoTableEntry) -> usize {
        let idx = self.0.len();
        self.0.push(entry);
        idx
    }

    pub fn push_writer(&mut self, writer: Box<dyn Write>) -> usize {
        self.push_entry(IoTableEntry::Write(writer))
    }

    pub fn push_reader(&mut self, reader: Box<dyn ReadSeek>) -> usize {
        self.push_entry(IoTableEntry::Read(reader))
    }
}

impl IoTableEntry {
    pub fn as_writer(&mut self) -> Option<&mut dyn Write> {
        match self {
            IoTableEntry::Write(w) => Some(w),
            _ => None,
        }
    }

    pub fn as_reader(&mut self) -> Option<&mut dyn ReadSeek> {
        match self {
            IoTableEntry::Read(r) => Some(r),
            _ => None,
        }
    }
}

fn with_io_table<T, F: FnOnce(&IoTable) -> T>(f: F) -> T {
    IO_TABLE.with_borrow(f)
}

fn with_io_table_mut<T, F: FnOnce(&mut IoTable) -> T>(f: F) -> T {
    IO_TABLE.with_borrow_mut(f)
}

pub fn fopen(path: &str, mode: &str) -> i32 {
    if mode.contains("w") {
        match path {
            "/dev/stdout" => {
                let idx = with_io_table_mut(|io| io.push_writer(Box::new(std::io::stdout())));
                idx as i32
            }
            "/dev/stderr" => {
                let idx = with_io_table_mut(|io| io.push_writer(Box::new(std::io::stderr())));
                idx as i32
            }
            _ => todo!("fopen write {}", path),
        }
    } else if mode.contains("r") {
        if let Ok(f) = std::fs::File::open(path) {
            let idx = with_io_table_mut(|io| io.push_reader(Box::new(f)));
            idx as i32
        } else {
            -1
        }
    } else {
        todo!("fopen mode {} (path: {})", mode, path)
    }
}

pub fn fwrite(fd: i32, data: &[u8]) -> i32 {
    with_io_table_mut(|io| {
        let idx = fd as u32 as usize;
        if idx >= io.0.len() {
            return 0;
        };
        let entry = &mut io.0[idx];
        if let Some(writer) = entry.as_writer() {
            let res = writer.write_all(data);
            match res {
                Ok(_) => data.len() as i32,
                Err(_) => 0,
            }
        } else {
            0
        }
    })
}

pub fn fread(fd: i32, dst: &mut [u8]) -> i32 {
    with_io_table_mut(|io| {
        let idx = fd as u32 as usize;
        if idx >= io.0.len() {
            return 0;
        }
        let entry = &mut io.0[idx];
        if let Some(reader) = entry.as_reader() {
            let res = reader.read_exact(dst);
            match res {
                Ok(_) => dst.len() as i32,
                Err(_) => 0,
            }
        } else {
            0
        }
    })
}

pub fn fclose(fd: i32) -> i32 {
    with_io_table_mut(|io| {
        let idx = fd as u32 as usize;
        if idx >= io.0.len() {
            return -1;
        }
        let entry = &mut io.0[idx];
        *entry = IoTableEntry::Closed;
        0
    })
}

pub fn fflush(fd: i32) -> i32 {
    with_io_table_mut(|io| {
        let idx = fd as u32 as usize;
        if idx >= io.0.len() {
            return -1;
        }
        let entry = &mut io.0[idx];
        if let Some(writer) = entry.as_writer() {
            writer.flush().map(|_| 0).unwrap_or(-1)
        } else {
            -1
        }
    })
}

pub fn ftell(fd: i32) -> i64 {
    with_io_table_mut(|io| {
        let idx = fd as u32 as usize;
        if idx >= io.0.len() {
            return -1;
        }
        let entry = &mut io.0[idx];
        if let Some(reader) = entry.as_reader() {
            reader.stream_position().map(|i| i as i64).unwrap_or(-1)
        } else {
            -1
        }
    })
}

pub fn fseek(fd: i32, offset: i64, whence: i32) -> i32 {
    with_io_table_mut(|io| {
        let idx = fd as u32 as usize;
        if idx >= io.0.len() {
            return 1;
        }
        let entry = &mut io.0[idx];
        if let Some(reader) = entry.as_reader() {
            let seekfrom = match whence {
                0 => SeekFrom::Start(offset as u64),
                1 => SeekFrom::Current(offset),
                2 => SeekFrom::End(offset),
                _ => return 1,
            };
            let res = reader.seek(seekfrom);
            res.map(|_| 0).unwrap_or(1)
        } else {
            1
        }
    })
}
