//! libvpx's VP9 encoder, for the measurements alone: what `screen-vp9`
//! 0.0.14 codes a 4:4:4 desktop with, set up as that crate sets it up, so
//! that the `*_against_libvpx` measurements in `src/encoder/mod.rs` play one
//! capture into both encoders, frame for frame and turn about. libvpx is a
//! dev-dependency and this module is not in a build of the crate.
//!
//! The setup, each line of it decided and recorded in `screen-vp9`: constant
//! quality with the quantizer pinned top and bottom by the same dial, no
//! lag, no dropped frame, no keyframe unasked, screen content, speed 7 (9 as
//! `Speed::Fastest`), no adaptive quantization, row threads, the tile
//! columns and the loop filter by this crate's own rules, and a frame's
//! rectangles as an active map of 16×16 blocks.

use std::os::raw::{c_int, c_ulong};

use vpx_sys as vpx;

use crate::{Picture, Rect, Speed, loop_filtered, quality_to_q, tile_columns_log2};

/// The side of a block of libvpx's active map.
const ACTIVE_BLOCK: usize = 16;

fn check(err: vpx::vpx_codec_err_t, call: &'static str) {
    // SAFETY: takes any code and returns a static string.
    let detail = unsafe { std::ffi::CStr::from_ptr(vpx::vpx_codec_err_to_string(err)) };
    assert!(err == vpx::vpx_codec_err_t_VPX_CODEC_OK, "vp9 {call}: {}", detail.to_string_lossy());
}

/// One stream of libvpx's at one picture size.
pub(crate) struct Libvpx {
    /// Boxed so that its address never changes.
    ctx: Box<vpx::vpx_codec_ctx_t>,
    cfg: vpx::vpx_codec_enc_cfg_t,
    img: vpx::vpx_image_t,
    size: (u16, u16),
    quality: u8,
    pts: i64,
    active: Vec<u8>,
    mapped: bool,
    begun: bool,
}

impl Libvpx {
    pub(crate) fn new(width: u16, height: u16, quality: u8, threads: usize) -> Self {
        let q = quality_to_q(quality);
        // SAFETY: a static interface; a config zeroed and then written by
        // libvpx before anything reads it; a zeroed context written through
        // by `enc_init_ver` at the ABI version of the linked headers; every
        // control's argument an `int`; and an image that wraps planes it is
        // handed at each encode and owns nothing.
        unsafe {
            let iface = vpx::vpx_codec_vp9_cx();
            assert!(!iface.is_null(), "this libvpx has no VP9 encoder");
            let mut cfg: vpx::vpx_codec_enc_cfg_t = std::mem::zeroed();
            check(vpx::vpx_codec_enc_config_default(iface, &mut cfg, 0), "config_default");
            cfg.g_w = u32::from(width);
            cfg.g_h = u32::from(height);
            cfg.g_profile = 1;
            cfg.g_threads = threads as u32;
            cfg.g_timebase.num = 1;
            cfg.g_timebase.den = 1000;
            cfg.g_lag_in_frames = 0;
            cfg.g_pass = vpx::vpx_enc_pass_VPX_RC_ONE_PASS;
            cfg.g_error_resilient = 0;
            cfg.kf_mode = vpx::vpx_kf_mode_VPX_KF_DISABLED;
            cfg.kf_max_dist = i32::MAX as u32;
            cfg.rc_end_usage = vpx::vpx_rc_mode_VPX_Q;
            cfg.rc_min_quantizer = q;
            cfg.rc_max_quantizer = q;
            cfg.rc_dropframe_thresh = 0;
            cfg.rc_resize_allowed = 0;
            let mut ctx: Box<vpx::vpx_codec_ctx_t> = Box::new(std::mem::zeroed());
            check(vpx::vpx_codec_enc_init_ver(&mut *ctx, iface, &cfg, 0, vpx::VPX_ENCODER_ABI_VERSION as c_int), "enc_init_ver");
            let blocks = usize::from(width).div_ceil(ACTIVE_BLOCK) * usize::from(height).div_ceil(ACTIVE_BLOCK);
            let mut encoder = Self { ctx, cfg, img: std::mem::zeroed(), size: (width, height), quality, pts: -1, active: vec![0; blocks], mapped: false, begun: false };
            let columns = tile_columns_log2(width, threads);
            encoder.control(vpx::vp8e_enc_control_id_VP8E_SET_CQ_LEVEL, q as c_int, "cq_level");
            encoder.control(vpx::vp8e_enc_control_id_VP9E_SET_TUNE_CONTENT, vpx::vp9e_tune_content_VP9E_CONTENT_SCREEN as c_int, "tune_content");
            encoder.control(vpx::vp8e_enc_control_id_VP8E_SET_CPUUSED, 7, "cpuused");
            encoder.control(vpx::vp8e_enc_control_id_VP9E_SET_COLOR_SPACE, vpx::vpx_color_space_VPX_CS_BT_601 as c_int, "color_space");
            encoder.control(vpx::vp8e_enc_control_id_VP9E_SET_COLOR_RANGE, vpx::vpx_color_range_VPX_CR_STUDIO_RANGE as c_int, "color_range");
            encoder.control(vpx::vp8e_enc_control_id_VP9E_SET_AQ_MODE, 0, "aq_mode");
            encoder.control(vpx::vp8e_enc_control_id_VP9E_SET_TILE_COLUMNS, columns as c_int, "tile_columns");
            encoder.control(vpx::vp8e_enc_control_id_VP9E_SET_DISABLE_LOOPFILTER, if loop_filtered(columns) { 0 } else { 2 }, "disable_loopfilter");
            if threads > 1 {
                encoder.control(vpx::vp8e_enc_control_id_VP9E_SET_ROW_MT, 1, "row_mt");
            }
            let wrapped = vpx::vpx_img_wrap(&mut encoder.img, vpx::vpx_img_fmt_VPX_IMG_FMT_I444, cfg.g_w, cfg.g_h, 1, std::ptr::dangling_mut::<u8>());
            assert!(!wrapped.is_null(), "libvpx refused a {width}x{height} picture");
            encoder
        }
    }

    pub(crate) fn quality(&self) -> u8 {
        self.quality
    }

    pub(crate) fn set_quality(&mut self, quality: u8) {
        let q = quality_to_q(quality);
        if q != quality_to_q(self.quality) {
            self.cfg.rc_min_quantizer = q;
            self.cfg.rc_max_quantizer = q;
            // SAFETY: the config libvpx validated at init with two fields
            // changed, on a live context.
            unsafe {
                check(vpx::vpx_codec_enc_config_set(&mut *self.ctx, &self.cfg), "enc_config_set");
                self.control(vpx::vp8e_enc_control_id_VP8E_SET_CQ_LEVEL, q as c_int, "cq_level");
            }
        }
        self.quality = quality;
    }

    pub(crate) fn set_speed(&mut self, speed: Speed) {
        // SAFETY: a live context and an `int`.
        unsafe { self.control(vpx::vp8e_enc_control_id_VP8E_SET_CPUUSED, if speed == Speed::Fastest { 9 } else { 7 }, "cpuused") };
    }

    /// Code `picture` onto the end of `out`, as `screen-vp9`'s encoder does:
    /// the rectangles of `changed` as the active map, none on a keyframe or
    /// the stream's first frame. `None` where libvpx made no frame of it,
    /// else whether the frame is a keyframe.
    pub(crate) fn encode(&mut self, picture: &Picture, keyframe: bool, changed: Option<&[Rect]>, out: &mut Vec<u8>) -> Option<bool> {
        assert_eq!(picture.size(), self.size);
        self.pts += 1;
        let flags: vpx::vpx_enc_frame_flags_t = if keyframe { vpx::VPX_EFLAG_FORCE_KF as vpx::vpx_enc_frame_flags_t } else { 0 };
        let (planes, strides) = (picture.planes(), picture.strides());
        self.map(changed.filter(|_| !keyframe && self.begun));
        // SAFETY: the planes are `picture`'s, borrowed for the call, at the
        // strides it reports; libvpx does not write to an input image. The
        // packets point into the encoder and are copied out before it is
        // called again.
        unsafe {
            for i in 0..3 {
                self.img.planes[i] = planes[i].as_ptr().cast_mut();
                self.img.stride[i] = strides[i] as c_int;
            }
            check(vpx::vpx_codec_encode(&mut *self.ctx, &self.img, self.pts, 1, flags, vpx::VPX_DL_REALTIME as c_ulong), "encode");
            let (from, mut key) = (out.len(), false);
            let mut iter: vpx::vpx_codec_iter_t = std::ptr::null();
            loop {
                let packet = vpx::vpx_codec_get_cx_data(&mut *self.ctx, &mut iter);
                if packet.is_null() {
                    break;
                }
                if (*packet).kind != vpx::vpx_codec_cx_pkt_kind_VPX_CODEC_CX_FRAME_PKT {
                    continue;
                }
                let frame = &(*packet).data.frame;
                out.extend_from_slice(std::slice::from_raw_parts(frame.buf.cast::<u8>(), frame.sz));
                key |= frame.flags & vpx::VPX_FRAME_IS_KEY != 0;
            }
            if out.len() == from {
                return None;
            }
            self.begun = true;
            Some(key)
        }
    }

    fn map(&mut self, changed: Option<&[Rect]>) {
        let (cols, rows) = (usize::from(self.size.0).div_ceil(ACTIVE_BLOCK), usize::from(self.size.1).div_ceil(ACTIVE_BLOCK));
        let active_map = match changed {
            Some(rects) => {
                self.active.fill(0);
                for rect in rects {
                    let (x, y) = (usize::from(rect.x), usize::from(rect.y));
                    let right = (x + usize::from(rect.width)).min(usize::from(self.size.0));
                    let bottom = (y + usize::from(rect.height)).min(usize::from(self.size.1));
                    if x >= right || y >= bottom {
                        continue;
                    }
                    let (first, last) = (x / ACTIVE_BLOCK, (right - 1) / ACTIVE_BLOCK);
                    for row in y / ACTIVE_BLOCK..=(bottom - 1) / ACTIVE_BLOCK {
                        self.active[row * cols + first..=row * cols + last].fill(1);
                    }
                }
                self.active.as_mut_ptr()
            }
            None if self.mapped => std::ptr::null_mut(),
            None => return,
        };
        let mut map = vpx::vpx_active_map { active_map, rows: rows as u32, cols: cols as u32 };
        // SAFETY: a live context and a map of a byte for each block libvpx
        // counts for this size, copied out before the call returns; a null
        // one takes the map back.
        check(unsafe { vpx::vpx_codec_control_(&mut *self.ctx, vpx::vp8e_enc_control_id_VP8E_SET_ACTIVEMAP as c_int, &mut map as *mut vpx::vpx_active_map) }, "active_map");
        self.mapped = changed.is_some();
    }

    unsafe fn control(&mut self, id: vpx::vp8e_enc_control_id, value: c_int, call: &'static str) {
        check(unsafe { vpx::vpx_codec_control_(&mut *self.ctx, id as c_int, value) }, call);
    }
}

impl Drop for Libvpx {
    fn drop(&mut self) {
        // SAFETY: the only handle, destroyed once; the wrapped image owns
        // nothing.
        unsafe {
            vpx::vpx_codec_destroy(&mut *self.ctx);
        }
    }
}
