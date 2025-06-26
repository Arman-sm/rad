use std::{cmp::min, io, sync::{Arc, Condvar, Mutex}};

use symphonia::core::io::MediaSource;

#[derive(Default)]
struct DataContainer {
    eof: bool,
    size: u64,
    bufs: Vec<Box<[u8]>>
}

#[derive(Default)]
pub struct DataLock {
    cnd: Condvar,
    lock: Mutex<DataContainer>
}

impl DataLock {
    pub fn add_buf(&self, buf: Box<[u8]>) {
        let mut lock = self.lock.lock().unwrap();
        
        assert!(!lock.eof);
        
        lock.size += buf.len() as u64;
        lock.bufs.push(buf);
        self.cnd.notify_all();
    }

    pub fn set_eof(&self) {
        let mut lock = self.lock.lock().unwrap();

        lock.eof = true;
    }
}

pub struct DynFmtBuf {
    data_lock: Arc<DataLock>,
    current_pos: u64,
    idx: usize,
    buf_idx: usize
}

impl io::Read for DynFmtBuf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut data = self.data_lock.lock.lock().unwrap();
        
        while data.bufs.len() <= self.idx {
            if data.eof {
                return Ok(0);
            }
            
            data = self.data_lock.cnd.wait(data).unwrap();
        }
        
        let data_buf = &data.bufs[self.idx];
        let bytes_read = min(data_buf.len() - self.buf_idx, buf.len());

        for i in 0..bytes_read {
            buf[i] = data_buf[self.buf_idx];
            self.buf_idx += 1;
        }

        if self.buf_idx == data_buf.len() {
            self.idx += 1;
            self.buf_idx = 0;
        }

        self.current_pos += bytes_read as u64;

        Ok(bytes_read)
    }
}

impl io::Seek for DynFmtBuf {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match pos {
            io::SeekFrom::Start(start) => self.seek_from_start(start),
            io::SeekFrom::Current(offset) => { self.seek_relative(offset)?; Ok(self.current_pos) },
            io::SeekFrom::End(offset) => {
                if 0 < offset {
                    return Err(io::ErrorKind::UnexpectedEof.into());
                }

                self.seek_from_end((offset * -1) as u64)
            }
        }
    }

    #[allow(unused)]
    fn seek_relative(&mut self, offset: i64) -> io::Result<()> {
        unimplemented!();
    }
}

impl DynFmtBuf {
    pub fn new() -> Self {
        DynFmtBuf {
            data_lock: Arc::new(Default::default()),
            current_pos: 0,
            buf_idx: 0,
            idx: 0
        }
    }

    pub fn data_lock(&self) -> Arc<DataLock> {
        self.data_lock.clone()
    }

    pub fn seek_from_start(&mut self, start: u64) -> io::Result<u64> {
        let lock = self.data_lock.lock.lock().unwrap();

        let mut set_idx = 0;
        let mut pos = 0;
        for buf in &lock.bufs {
            if (0..buf.len() as u64).contains(&(start - pos)) {
                self.idx = set_idx;
                self.buf_idx = (start - pos) as usize;
                self.current_pos = start;

                return Ok(start);
            }
            
            set_idx += 1;
            pos += buf.len() as u64;
        }

        Err(io::ErrorKind::UnexpectedEof.into())
    }

    pub fn seek_from_end(&mut self, offset: u64) -> io::Result<u64> {
        let lock = self.data_lock.lock.lock().unwrap();

        if lock.size == 0 || (lock.size as u64) < offset {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        let mut set_idx = lock.bufs.len() - 1;
        let mut pos = lock.size;
        for buf in lock.bufs.iter().rev() {
            if ((pos - buf.len() as u64)..pos).contains(&(lock.size - offset)) {
                self.idx = set_idx;
                self.buf_idx = (offset - (lock.size - pos)) as usize;
                self.current_pos = lock.size - offset;

                return Ok(self.current_pos as u64);
            }
            
            set_idx -= 1;
            pos -= buf.len() as u64;
        }

        unreachable!();
    }
}

impl MediaSource for DynFmtBuf {
    fn is_seekable(&self) -> bool { true }
    fn byte_len(&self) -> Option<u64> {
        let lock =  self.data_lock.lock.lock().unwrap();
        if lock.eof {
            return Some(lock.size as u64);
        }

        None
    }
}