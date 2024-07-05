// use std::{mem::size_of, net::{SocketAddr, TcpListener}, slice::from_raw_parts, sync::{atomic::AtomicBool, Arc, Mutex}, thread};

// use rad_compositor::{adapter::AdapterHandle, compositor::CompositionBufferNode};
// use tungstenite::{accept, Message};

// /// Size of each buffer in bytes
// const BUF_SIZE: usize  = 4096;

// pub fn init_ws_adapter(id: String, bind_addr: SocketAddr, mut cmp_node: Arc<CompositionBufferNode<{BUF_SIZE / size_of::<f32>()}>>) -> AdapterHandle {
//     let socket = TcpListener::bind(bind_addr).unwrap();
//     let status = Arc::new(Mutex::new("Established".to_owned()));
//     let is_closed = Arc::new(AtomicBool::new(false));

//     thread::Builder::new().name(format!("ap-ws-{}", id)).spawn(move || {
//         for st in socket.incoming() {
//             let mut ws = accept(st.unwrap()).unwrap();
            
//             loop {
//                 {
//                     let buf = cmp_node.buf();
//                     let buf_u8 = unsafe { from_raw_parts(buf.as_ptr() as *const u8, buf.len() * 4) };
//                     let msg = Message::binary(buf_u8.to_vec());
                    
//                     if let Err(_) = ws.send(msg) {
//                         break;
//                     }
//                 }

//                 cmp_node = cmp_node.next();
//             }
//         }
//     }).unwrap();

//     AdapterHandle::new(id, "net-ws".to_owned(), status, is_closed)
// }