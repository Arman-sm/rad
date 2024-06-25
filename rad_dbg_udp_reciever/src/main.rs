use std::{mem::{size_of, transmute}, net::UdpSocket, thread};
use kanal::{Receiver, Sender};

// TODO: Make these configurable via command line arguments
const BIND_ADDR: &str = ""; // e.g. 0.0.0.0:5050
const RECV_ADDR: &str = ""; // e.g. 127.0.0.1:5051

const BUF_SIZE: usize = 4096; // 512 frames = 4096 / channels / size of f32 in bytes
const CHANNELS: usize = 2;

pub struct Writer {
    rx: Receiver<[f32; BUF_SIZE / size_of::<f32>()]>,
    buf: [f32; BUF_SIZE / size_of::<f32>()],
    frame_offset: usize,
}

// TODO: Make these configurable
impl rodio::Source for Writer {
	fn channels(&self) -> u16 { CHANNELS as _ }
	fn current_frame_len(&self) -> Option<usize> { None }
	fn sample_rate(&self) -> u32 { 44100 }
	fn total_duration(&self) -> Option<std::time::Duration> { None }
}

impl Iterator for Writer {
	type Item = f32;

	fn next(&mut self) -> Option<f32> {
		if self.frame_offset == 0 {
            println!("PLAYBACK: Receiving buffer");
            self.buf = self.rx.recv().unwrap();
		}
        
		let res = self.buf[self.frame_offset];
        
		self.frame_offset += 1;
        
		if self.frame_offset == BUF_SIZE / size_of::<f32>() {
            self.frame_offset = 0;
		}
        
		Some(res)
	}
}

fn stream_thread(socket: UdpSocket, tx: Sender<[f32; BUF_SIZE / size_of::<f32>()]>) {
    loop {
        println!("SOCKET: Receiving buffer");
        
        #[allow(deprecated, invalid_value)]
        let mut buf: [u8; BUF_SIZE] = unsafe { std::mem::uninitialized() };
        socket.recv(&mut buf).unwrap();

        tx.send(unsafe { transmute(buf) }).unwrap();
    }
}

fn main() {
    let (_out, out_handle) = rodio::OutputStream::try_default().unwrap();
    let socket = UdpSocket::bind(BIND_ADDR).unwrap();

    socket.connect(RECV_ADDR).unwrap();

    let (tx, rx) = kanal::unbounded::<[f32; BUF_SIZE / size_of::<f32>()]>();

    thread::spawn(move || {
        stream_thread(socket, tx)
    });

    let writer = Writer {
        buf: [0.0; BUF_SIZE / size_of::<f32>()],
        frame_offset: 0,
        rx
    };

    out_handle.play_raw(writer).unwrap();

    loop {
        // sleep(Duration::from_secs(60))
    }
}
