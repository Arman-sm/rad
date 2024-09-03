use std::{io::{BufRead, Read, Write}, net::{SocketAddr, TcpListener, TcpStream}, slice::from_raw_parts, sync::{atomic::AtomicBool, Arc, Mutex}, thread};

use rad_compositor::{adapter::AdapterHandle, compositor::CompositionBufferNode};
use crate::utils::wav::gen_wav_header;

fn net_err_log(err: std::io::Error) { log::debug!("Network error occurred: {err:?}"); }
macro_rules! net_err_handle {
    ($err:expr) => {{
        if let Err(e) = $err {
            net_err_log(e);
        }
    }};
}

/// Size of each buffer in bytes
const BUF_SIZE: usize = 2048;
const BUF_SIZE_HEX: &str = "1000"; // format!("{:x}", BUF_SIZE)

type TCmpNode = Arc<CompositionBufferNode<1024>>;

const HTTP_INITIAL_MSG: &str = "HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\nConnection: keep-alive\r\nKeep-Alive: timeout=5\r\nTransfer-Encoding: chunked\r\n\r\n";

// TODO: Optimize
pub fn stream_as_wav(cmp_node: &mut TCmpNode, sample_rate: u32, channels: u16, mut st: TcpStream) {
    // Streams the data using http chunked streaming method
    // Reference: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Transfer-Encoding
    // Reference: Analyzing the same thing done in https://github.com/Arman-sm/Atmosphere via wireshark 

    // Initial message: Information about the type of response along with the initial part of the wav file describing it.
    // TODO: Maybe do compressing if specified.

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
            log::debug!("Network error occurred: {e:?}");
            return;
        }

        st.flush().unwrap();

        *cmp_node = cmp_node.next();
    }
}


const HTTP_200_RESPONSE: &str = "HTTP/1.1 200 OK\r\n";
const HTTP_400_RESPONSE: &str = "HTTP/1.1 400 Bad Request\r\n";
fn handle_conn(mut st: TcpStream, sample_rate: u32, channels: u16, cmp_node: &mut TCmpNode) {
    macro_rules! static_file_serve {
        ($path:expr, $mime_type:expr) => {{
            log::debug!("Sending '{}'.", $path);

            let data = include_bytes!($path);

            net_err_handle!(st.write_all(HTTP_200_RESPONSE.as_bytes()));
            net_err_handle!(st.write_all(format!("Content-Type: {}\r\nContent-Length: {}\r\n\r\n", $mime_type, data.len()).as_bytes()));
            net_err_handle!(st.write_all(data));
        }};
    }

    let mut buf = [0u8; 4096];
    st.read(&mut buf).unwrap();
    
    log::debug!("Parsing incoming http request.");

    let req_line = match buf.lines().next() {
        Some(Ok(line)) => line,
        _ => {
            log::debug!("Couldn't read the request line of the request.");
            net_err_handle!(st.write_all(HTTP_400_RESPONSE.as_bytes()));
            return;
        }
    };

    let uri = match req_line.split(' ').nth(1) {
        Some(url) => url,
        None => {
            log::debug!("Couldn't parse the request line of the request.");
            net_err_handle!(st.write_all(HTTP_400_RESPONSE.as_bytes()));
            return;
        }
    };

    log::debug!("Successfully read the requested URI '{uri}'.");

    match uri {
        "/audio.wav" => {
            log::debug!("Sending the audio as wav.");

            *cmp_node = cmp_node.live(sample_rate, channels);
            
            let mut cmp = cmp_node.clone();
        
            thread::spawn(move || {
                stream_as_wav(&mut cmp, sample_rate, channels, st);
            });
        },
        "/"            => { static_file_serve!("./simple_http_static/index.html", "text/html"); },
        //// "/simple.svg"  => { static_file_serve!("./simple_http_static/simple.svg", "image/svg+xml"); }, // Heavy!
        "/simple.png"  => { static_file_serve!("./simple_http_static/simple.png", "image/png"); },
        "/favicon.png" => { static_file_serve!("./simple_http_static/favicon.png", "image/png"); },
        // TODO: Optimize SVG file and use that instead
        "/logo.png"    => { static_file_serve!("./simple_http_static/logo.png", "image/png"); },
        _ => {
            log::debug!("The requested URI is invalid.");
            net_err_handle!(st.write_all(HTTP_400_RESPONSE.as_bytes()));
            return;
        }
    }
}

pub fn init_simple_http_adapter(id: String, sample_rate: u32, channels: u16, bind_addr: SocketAddr, mut cmp_node: TCmpNode) -> AdapterHandle {
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
                    handle_conn(st, sample_rate, channels, &mut cmp_node);
                },
                Err(e) => {
                    log::error!("Connection failure happened in adapter '{_id}' with error '{:?}'.", e);
                }
            }
        }
    }).unwrap();

    AdapterHandle::new(id, "net-simple-http".to_owned(), status.clone(), _is_closed)
}