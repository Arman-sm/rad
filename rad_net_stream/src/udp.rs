use std::{mem::size_of, net::{SocketAddr, UdpSocket}, slice::from_raw_parts, sync::{atomic::AtomicBool, Arc, Mutex}, thread::{self}};

use rad_compositor::{adapter::AdapterHandle, compositor::CompositionBufferNode};

// TODO: Allow customization of the buffer size, channel count, and sample rate.

/// Size of each buffer in bytes
const BUF_SIZE: usize  = 4096;
// const CHANNELS: usize  = 2;

pub fn init_udp_adapter(id: String, bind_addr: SocketAddr, dest_addr: SocketAddr, mut cmp_node: Arc<CompositionBufferNode<{BUF_SIZE / size_of::<f32>()}>>) -> AdapterHandle {
    let socket = UdpSocket::bind(bind_addr).unwrap();
    let status = Arc::new(Mutex::new("Established".to_owned()));
    let is_closed = Arc::new(AtomicBool::new(false));

    let _is_closed = is_closed.clone();
    thread::Builder::new().name(format!("ap-udp-{id}")).spawn(move || loop {
        use std::sync::atomic::Ordering;

        if is_closed.load(Ordering::Relaxed) { return; }

        let buf: &[f32; BUF_SIZE / size_of::<f32>()] = cmp_node.buf();
    
        let socket_res = socket.send_to(
            unsafe { from_raw_parts(buf.as_ptr() as *const _, buf.len() * 4) },
            dest_addr
        );
        
        if let Err(e) = socket_res {
            eprintln!("UDP send failed: {:?}", e)
        }
        
        cmp_node = cmp_node.next();
    }).unwrap();

    AdapterHandle::new(id, "net-udp".to_owned(), status.clone(), _is_closed)
}