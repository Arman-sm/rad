use rad_compositor::{composition::CompositionState, source::{formatted::{FormattedStreamSource, StreamOrigin}, utils::dyn_buf::DynFmtBuf}};
use actix_web::{get, post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use futures::StreamExt;

use crate::State;

/// Handy macro for finding the right composition and handling errors related to it.
macro_rules! find_cmp_read {
    ($cmp_reg:expr, $id:expr) => {
        match $cmp_reg.find_composition(&$id) {
            Some(c) => c.read().unwrap(),
            None => { return HttpResponse::NotFound().finish(); }
        }
    };
}

/// Handy macro for finding the right composition and handling errors related to it.
macro_rules! find_cmp_write {
    ($cmp_reg:expr, $id:expr) => {
        match $cmp_reg.find_composition(&$id) {
            Some(c) => c.write().unwrap(),
            None => { return HttpResponse::NotFound().body("NOT FOUND\n"); }
        }
    };
}

/// This is the representational type of the composition state that is going to be exposed through the API.
#[derive(Deserialize, Serialize)]
struct SerdeCompositor {
    pub id: String,
	pub is_paused: bool,
	pub channels: usize,
	// pub sources: Vec<CompositionSrc>,
	pub amplification: f32,
    pub time: f32
}

impl SerdeCompositor {
    fn from_state(value: &CompositionState) -> Self {
        SerdeCompositor {
            id: value.get_id().clone(),
            is_paused: value.is_paused(),
            channels: value.get_channels().into(),
            amplification: value.get_amplification(),
            time: value.get_time_sec() as f32
        }
    }
}

#[post("/{cmp_id}/time")]
pub async fn set_time(body: web::Bytes, _cmp_id: web::Path<(String,)>, data: web::Data<State>) -> impl Responder {
    let (cmp_id,) = _cmp_id.into_inner();

    let time_str = match String::from_utf8(body.to_vec()) {
        Ok(st) => st,
        Err(_) => { return HttpResponse::BadRequest().body("BAD REQUEST ( INVALID TIME )\n"); }
    };

    let time = match time_str.parse::<f32>() {
        Ok(v) => v,
        Err(_) => { return HttpResponse::BadRequest().body("BAD REQUEST ( INVALID TIME )\n"); }
    };

    if time < 0.0 {
        return HttpResponse::BadRequest().body("BAD REQUEST ( NEGATIVE TIME )\n");
    }

    let cmp_reg = data.cmp_reg.lock().unwrap();
    let mut cmp = find_cmp_write!(cmp_reg, cmp_id);
    
    cmp.set_time_millis((time * 1000.0) as u64);

    HttpResponse::Ok().body("OK\n")
}

#[get("/{cmp_id}")]
pub async fn get_info(_cmp_id: web::Path<(String,)>, data: web::Data<State>) -> impl Responder {
    let (cmp_id,) = _cmp_id.into_inner();

    let cmp_reg = data.cmp_reg.lock().unwrap();
    let cmp = find_cmp_read!(cmp_reg, cmp_id);
    
    let toml_ser = toml::to_string(&SerdeCompositor::from_state(&cmp)).unwrap();
    HttpResponse::Ok().body(toml_ser)
}

#[get("/{cmp_id}/json")]
pub async fn get_info_json(_cmp_id: web::Path<(String,)>, data: web::Data<State>) -> impl Responder {
    let (cmp_id,) = _cmp_id.into_inner();

    let cmp_reg = data.cmp_reg.lock().unwrap();
    let cmp = find_cmp_read!(cmp_reg, cmp_id);

    HttpResponse::Ok().json(SerdeCompositor::from_state(&cmp))  
}

#[post("/{cmp_id}/pause")]
pub async fn set_pause(_cmp_id: web::Path<(String,)>, data: web::Data<State>) -> impl Responder {
    let (cmp_id,) = _cmp_id.into_inner();

    let cmp_reg = data.cmp_reg.lock().unwrap();
    let mut cmp = find_cmp_write!(cmp_reg, cmp_id);
    cmp.set_paused(true);

    HttpResponse::Ok().body("OK\n")
}

#[post("/{cmp_id}/play")]
pub async fn set_play(_cmp_id: web::Path<(String,)>, data: web::Data<State>) -> impl Responder {
    let (cmp_id,) = _cmp_id.into_inner();

    let cmp_reg = data.cmp_reg.lock().unwrap();
    let mut cmp = find_cmp_write!(cmp_reg, cmp_id);
    cmp.set_paused(true);

    HttpResponse::Ok().body("OK\n")
}

#[post("/{cmp_id}/upload")]
pub async fn upload(_cmp_id: web::Path<(String,)>, data: web::Data<State>, mut payload: web::Payload) -> impl Responder {
    let (cmp_id,) = _cmp_id.into_inner();
    
    let dyn_buf = Box::new(DynFmtBuf::new());
    let dyn_buf_data = dyn_buf.data_lock();

    let mut byte_counter = 0;
    while let Some(buf) = payload.next().await {
        let buf =
            match buf {
                Ok(b) => b,
                Err(e) => { eprintln!("Failed to receive file: {}", e); return HttpResponse::InternalServerError().finish(); },
            };
    
        let v = buf.into_iter().collect::<Vec<_>>();
        byte_counter += v.len();
        
        dyn_buf_data.add_buf(v.into_boxed_slice());

        if 512 < byte_counter { break; }
    }

    let src = FormattedStreamSource::open_stream(dyn_buf, Some(StreamOrigin::RemoteClient));

    let src = 
        match src {
            Some(s) => s,
            None => { return HttpResponse::InternalServerError().body("INTERNAL SERVER ERROR\n"); }
        };
    
    {
        let cmp_reg = data.cmp_reg.lock().unwrap();
        let mut cmp = find_cmp_write!(cmp_reg, cmp_id);

        cmp.push_src_default(src.into());
    }

    while let Some(buf) = payload.next().await {
        let buf =
            match buf {
                Ok(b) => b,
                Err(e) => { eprintln!("Failed to receive file: {}", e); return HttpResponse::InternalServerError().finish(); },
            };
    
        let v = buf.into_iter().collect::<Vec<_>>();
        dyn_buf_data.add_buf(v.into_boxed_slice());
    }

    dyn_buf_data.set_eof();

    HttpResponse::Ok().body("OK\n")
}