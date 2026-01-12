use std::{
    cell::RefCell,
    io::{Read, Write},
};

thread_local! {
    static IO_TABLE: RefCell<IoTable> = RefCell::new(IoTable::new());
}

pub trait ReadWrite: Read + Write {}

impl<T: Read + Write> ReadWrite for T {}

pub enum IoTableEntry {
    Read(Box<dyn Read>),
    Write(Box<dyn Write>),
    ReadWrite(Box<dyn ReadWrite>),
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
}

impl IoTableEntry {
    pub fn as_writer(&mut self) -> Option<&mut dyn Write> {
        match self {
            IoTableEntry::Write(w) => Some(w),
            IoTableEntry::ReadWrite(w) => Some(w),
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
    match path {
        "/dev/stdout" => {
            let idx = with_io_table_mut(|io| io.push_writer(Box::new(std::io::stdout())));
            idx as i32
        }
        "/dev/stderr" => {
            let idx = with_io_table_mut(|io| io.push_writer(Box::new(std::io::stderr()))) as i32;
            idx as i32
        }
        _ => todo!("fopen {}", path),
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
