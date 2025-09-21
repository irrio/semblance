use super::{
    WasmDataInst, WasmElemInst, WasmFuncInst, WasmGlobalInst, WasmMemInst, WasmModuleInst,
    WasmTableInst,
};

#[derive(Debug, Clone, Copy)]
pub struct WasmInstanceAddr(u32);
#[derive(Debug, Clone, Copy)]
pub struct WasmFuncAddr(u32);
#[derive(Debug, Clone, Copy)]
pub struct WasmTableAddr(u32);
#[derive(Debug, Clone, Copy)]
pub struct WasmMemAddr(u32);
#[derive(Debug, Clone, Copy)]
pub struct WasmGlobalAddr(u32);
#[derive(Debug, Clone, Copy)]
pub struct WasmElemAddr(u32);
#[derive(Debug, Clone, Copy)]
pub struct WasmDataAddr(u32);

pub trait ToIdx {
    fn to_idx(self) -> usize;
}

pub trait FromIdx {
    fn from_idx(idx: usize) -> Self;
}

#[inline(always)]
fn to_idx_nullable(addr: u32) -> usize {
    (addr as usize) - 1
}

#[inline(always)]
fn from_idx_nullable(idx: usize) -> u32 {
    (idx as u32) + 1
}

impl ToIdx for WasmInstanceAddr {
    fn to_idx(self) -> usize {
        self.0 as usize
    }
}

impl FromIdx for WasmInstanceAddr {
    fn from_idx(idx: usize) -> Self {
        WasmInstanceAddr(idx as u32)
    }
}

impl ToIdx for WasmFuncAddr {
    fn to_idx(self) -> usize {
        to_idx_nullable(self.0)
    }
}

impl FromIdx for WasmFuncAddr {
    fn from_idx(idx: usize) -> Self {
        Self(from_idx_nullable(idx))
    }
}

impl ToIdx for WasmTableAddr {
    fn to_idx(self) -> usize {
        to_idx_nullable(self.0)
    }
}

impl FromIdx for WasmTableAddr {
    fn from_idx(idx: usize) -> Self {
        Self(from_idx_nullable(idx))
    }
}

impl ToIdx for WasmMemAddr {
    fn to_idx(self) -> usize {
        to_idx_nullable(self.0)
    }
}

impl FromIdx for WasmMemAddr {
    fn from_idx(idx: usize) -> Self {
        Self(from_idx_nullable(idx))
    }
}

impl ToIdx for WasmGlobalAddr {
    fn to_idx(self) -> usize {
        to_idx_nullable(self.0)
    }
}

impl FromIdx for WasmGlobalAddr {
    fn from_idx(idx: usize) -> Self {
        Self(from_idx_nullable(idx))
    }
}

impl ToIdx for WasmElemAddr {
    fn to_idx(self) -> usize {
        to_idx_nullable(self.0)
    }
}

impl FromIdx for WasmElemAddr {
    fn from_idx(idx: usize) -> Self {
        Self(from_idx_nullable(idx))
    }
}

impl ToIdx for WasmDataAddr {
    fn to_idx(self) -> usize {
        to_idx_nullable(self.0)
    }
}

impl FromIdx for WasmDataAddr {
    fn from_idx(idx: usize) -> Self {
        Self(from_idx_nullable(idx))
    }
}

impl WasmFuncAddr {
    pub const NULL: WasmFuncAddr = WasmFuncAddr(0);
}

pub trait Addressable {
    type Addr: ToIdx + FromIdx;
}

impl<'wmod> Addressable for WasmModuleInst<'wmod> {
    type Addr = WasmInstanceAddr;
}

impl<'wmod> Addressable for WasmFuncInst<'wmod> {
    type Addr = WasmFuncAddr;
}

impl<'wmod> Addressable for WasmTableInst<'wmod> {
    type Addr = WasmTableAddr;
}

impl<'wmod> Addressable for WasmMemInst<'wmod> {
    type Addr = WasmMemAddr;
}

impl<'wmod> Addressable for WasmGlobalInst<'wmod> {
    type Addr = WasmGlobalAddr;
}

impl Addressable for WasmElemInst {
    type Addr = WasmElemAddr;
}

impl<'wmod> Addressable for WasmDataInst<'wmod> {
    type Addr = WasmDataAddr;
}

pub struct StoreTable<T: Addressable> {
    items: Vec<T>,
}

impl<T: Addressable> StoreTable<T> {
    pub fn new() -> Self {
        StoreTable { items: Vec::new() }
    }

    pub fn with_capacity(cap: u32) -> Self {
        StoreTable {
            items: Vec::with_capacity(cap as usize),
        }
    }

    pub fn add(&mut self, item: T) -> T::Addr {
        let idx = self.items.len();
        self.items.push(item);
        T::Addr::from_idx(idx)
    }

    pub fn resolve(&self, addr: T::Addr) -> &T {
        &self.items[addr.to_idx()]
    }

    pub fn try_resolve(&self, addr: T::Addr) -> Option<&T> {
        self.items.get(addr.to_idx())
    }

    pub fn resolve_mut(&mut self, addr: T::Addr) -> &mut T {
        &mut self.items[addr.to_idx()]
    }

    pub fn resolve_multi_mut(&mut self, addr1: T::Addr, addr2: T::Addr) -> (&mut T, &mut T) {
        let idx1 = addr1.to_idx();
        let idx2 = addr2.to_idx();
        assert!(idx1 != idx2);
        let mid = (idx1 + idx2) / 2;
        let (low, high) = self.items.split_at_mut(mid);
        if idx1 > idx2 {
            let ref1 = &mut high[idx1 - mid];
            let ref2 = &mut low[idx2];
            (ref1, ref2)
        } else {
            let ref1 = &mut low[idx1];
            let ref2 = &mut high[idx2 - mid];
            (ref1, ref2)
        }
    }
}
