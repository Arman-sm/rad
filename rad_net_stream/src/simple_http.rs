use std::{io::{Read, Write}, net::{SocketAddr, TcpListener, TcpStream}, slice::from_raw_parts, sync::{atomic::AtomicBool, Arc, Mutex}, thread};

use rad_compositor::{adapter::AdapterHandle, compositor::CompositionBufferNode};
use wav::gen_wav_header;

mod wav;

/// Size of each buffer in bytes
const BUF_SIZE: usize = 2048;
const BUF_SIZE_HEX: &str = "1000"; // format!("{:x}", BUF_SIZE)

type TCmpNode = Arc<CompositionBufferNode<1024>>;

const HTTP_INITIAL_MSG: &str = "HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\nConnection: keep-alive\r\nKeep-Alive: timeout=5\r\nTransfer-Encoding: chunked\r\n\r\n";

// TODO: Optimize
fn handle_conn(cmp_node: &mut TCmpNode, sample_rate: u32, channels: u16, mut st: TcpStream) {
    // Streams the data using http chunked streaming method
    // Reference: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Transfer-Encoding
    // Reference: Analyzing the same thing done in https://github.com/Arman-sm/Atmosphere via wireshark 

    // Initial message: Information about the type of response along with the initial part of the wav file describing it.
    let mut buf = [0u8; 4096];
    st.read(&mut buf).unwrap();

    // TODO: Parse the request and respond accordingly. (Maybe do compressing if specified)

    let mut wav_header = gen_wav_header(sample_rate, channels);
    let buf_len_hex = format!("{:x}\r\n", wav_header.len());
    let mut buf = Vec::new();

    buf.reserve_exact(
        HTTP_INITIAL_MSG.as_bytes().len() + 
        buf_len_hex.as_bytes().len() + 
        wav_header.len() +
        2 // The '\r\n' at the ent
    );

    buf.extend_from_slice(HTTP_INITIAL_MSG.as_bytes());
    buf.extend_from_slice(buf_len_hex.as_bytes());
    buf.append(&mut wav_header);
    buf.push('\r' as u8);
    buf.push('\n' as u8);

    st.write_all(&buf).unwrap();
    st.flush().unwrap();
    
    // Getting to the head
    *cmp_node = cmp_node.head();

    // Sending the actual audio
    loop {
        let buf_f32: &[f32; 1024] = cmp_node.buf();

        let mut audio_i16 = [0i16; BUF_SIZE / 2];
        for (i, v) in buf_f32.iter().enumerate() {
            audio_i16[i] = (v.min(1.0).max(-1.0) * i16::MAX as f32) as i16;
        }

        let audio_bytes = unsafe { from_raw_parts(audio_i16.as_ptr() as *const u8, BUF_SIZE) };
        
        let mut buf = Vec::with_capacity(BUF_SIZE + BUF_SIZE_HEX.as_bytes().len() + "\r\n".as_bytes().len() * 2);

        buf.extend_from_slice(format!("{:x}\r\n", BUF_SIZE).as_bytes());
        buf.extend_from_slice(audio_bytes);
        buf.extend_from_slice("\r\n".as_bytes());
        
        if let Err(e) = st.write_all(&buf) {
            eprintln!("Network error occurred: {e:?}");
            return;
        }

        st.flush().unwrap();

        *cmp_node = cmp_node.next();
    }
}

pub fn init_simple_http_adapter(id: String, sample_rate: u32, channels: u16, bind_addr: SocketAddr, cmp_node: TCmpNode) -> AdapterHandle {
    let socket = TcpListener::bind(bind_addr).unwrap();
    let status = Arc::new(Mutex::new("Established".to_owned()));
    let is_closed = Arc::new(AtomicBool::new(false));

    let _is_closed = is_closed.clone();

    let _id = id.clone();
    thread::Builder::new().name(format!("ap-simple-http-{id}")).spawn(move || {
        use std::sync::atomic::Ordering;

        for incoming in socket.incoming() {
            if is_closed.load(Ordering::Relaxed) { return; }
            match incoming {
                Ok(st) => {
                    let mut cmp = cmp_node.clone();
                    thread::spawn(move || {
                        handle_conn(&mut cmp, sample_rate, channels, st);
                    });
                },
                Err(e) => {
                    log::error!("Connection failure happened in adapter '{_id}' with error '{:?}'.", e);
                }
            }

        }
    }).unwrap();

    AdapterHandle::new(id, "net-simple-http".to_owned(), status.clone(), _is_closed)
}