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
