use core;
use super::super::alloc;
use super::super::alloc::{SliceWrapper, SliceWrapperMut};
use super::interface;
use super::backward_references::BrotliEncoderParams;
use super::input_pair::{InputPair, InputReference, InputReferenceMut};
use super::ir_interpret::{IRInterpreter, push_base};
use super::util::{floatX, FastLog2u16};
use super::find_stride;
use core::simd::{i16x16, f32x8};
// the high nibble, followed by the low nibbles
pub const CONTEXT_MAP_PRIOR_SIZE: usize = 256 * 17;
pub const STRIDE_PRIOR_SIZE: usize = 256 * 256 * 2;
pub const ADV_PRIOR_SIZE: usize = 256 * 256 * 16 * 2;
pub const DEFAULT_SPEED: (u16, u16) = (8, 8192);
pub enum WhichPrior {
    CM = 0,
    ADV = 1,
    SLOW_CM = 2,
    FAST_CM = 3,
    STRIDE1 = 4,
    STRIDE2 = 5,
    STRIDE3 = 6,
    STRIDE4 = 7,
//    STRIDE8 = 8,
    NUM_PRIORS = 8,
    // future ideas
}

pub trait Prior {
    fn lookup_lin(stride_byte: u8, selected_context:u8, actual_context:usize, high_nibble: Option<u8>) -> usize;
    #[inline]
    fn lookup_mut(data:&mut [i16x16], stride_byte: u8, selected_context:u8, actual_context:usize, high_nibble: Option<u8>) -> CDF {
        let index = Self::lookup_lin(stride_byte, selected_context, actual_context,
                             high_nibble);
        CDF::from(&mut data[index])
    }
    #[inline]
    fn lookup(data:&[i16x16], stride_byte: u8, selected_context:u8, actual_context:usize, high_nibble: Option<u8>) -> &i16x16 {
        let index = Self::lookup_lin(stride_byte, selected_context, actual_context,
                             high_nibble);
        &data[index]
    }
    #[allow(unused_variables)]
    #[inline]
    fn score_index(stride_byte: u8, selected_context: u8, actual_context: usize, high_nibble: Option<u8>) -> usize {
        let which = Self::which() as usize;
        assert!(which < WhichPrior::NUM_PRIORS as usize);
        assert!(actual_context < 256);
        if let Some(nibble) = high_nibble {
            WhichPrior::NUM_PRIORS as usize * (actual_context + 4096 + 256 * nibble as usize) + which
        } else {
            WhichPrior::NUM_PRIORS as usize * (actual_context + 256 * (stride_byte >> 4) as usize) + which
        }
    }
    fn which() -> WhichPrior;
}


fn upper_score_index(stride_byte: u8, selected_context: u8, actual_context: usize) -> usize {
  actual_context + 256 * (stride_byte >> 4) as usize
}
fn lower_score_index(stride_byte: u8, selected_context: u8, actual_context: usize, high_nibble: u8) -> usize {
  actual_context + 4096 + 256 * high_nibble as usize
}


#[allow(unused_variables)]
#[inline]
fn stride_lookup_lin(stride_byte:u8, selected_context:u8, actual_context:usize, high_nibble: Option<u8>) -> usize {
    if let Some(nibble) = high_nibble {
        1 + 2 * (actual_context as usize
                 | ((stride_byte as usize & 0xf) << 8)
                 | ((nibble as usize) << 12))
    } else {
        2 * (actual_context as usize | ((stride_byte as usize) << 8))
    }
}
pub struct Stride1Prior{
}
impl Stride1Prior {
    pub fn offset() -> usize{
        0
    }
}

impl Prior for Stride1Prior {
    #[inline]
    fn lookup_lin(stride_byte:u8, selected_context:u8, actual_context:usize, high_nibble: Option<u8>) -> usize {
        stride_lookup_lin(stride_byte, selected_context, actual_context, high_nibble)
    }
    #[inline]
    fn which() -> WhichPrior {
        WhichPrior::STRIDE1
    }
}
/*impl StridePrior for Stride1Prior {
    const STRIDE_OFFSET:usize = 0;
}*/
pub struct Stride2Prior{
}
impl Stride2Prior {
    #[inline(always)]
    pub fn offset() -> usize{
        1
    }
}

impl Prior for Stride2Prior {
    #[inline(always)]
    fn lookup_lin(stride_byte:u8, selected_context:u8, actual_context:usize, high_nibble: Option<u8>) -> usize {
        stride_lookup_lin(stride_byte, selected_context, actual_context, high_nibble)
    }
    #[inline]
    fn which() -> WhichPrior {
        WhichPrior::STRIDE2
    }
}
/*impl StridePrior for Stride2Prior {
    const STRIDE_OFFSET:usize = 1;
}*/
pub struct Stride3Prior{
}
impl Stride3Prior {
    #[inline(always)]
    pub fn offset() -> usize{
        2
    }
}

impl Prior for Stride3Prior {
    #[inline(always)]
    fn lookup_lin(stride_byte:u8, selected_context:u8, actual_context:usize, high_nibble: Option<u8>) -> usize {
        stride_lookup_lin(stride_byte, selected_context, actual_context, high_nibble)
    }
    #[inline]
    fn which() -> WhichPrior {
        WhichPrior::STRIDE3
    }
}

/*impl StridePrior for Stride3Prior {
    const STRIDE_OFFSET:usize = 2;
}*/
pub struct Stride4Prior{
}
impl Stride4Prior {
    #[inline(always)]
    pub fn offset() -> usize{
        3
    }
}
impl Prior for Stride4Prior {
    #[inline(always)]
    fn lookup_lin(stride_byte:u8, selected_context:u8, actual_context:usize, high_nibble: Option<u8>) -> usize {
        stride_lookup_lin(stride_byte, selected_context, actual_context, high_nibble)
    }
    #[inline]
    fn which() -> WhichPrior {
        WhichPrior::STRIDE4
    }
}

/*impl StridePrior for Stride4Prior {
    const STRIDE_OFFSET:usize = 3;
}*/
/*pub struct Stride8Prior{
}
impl StridePrior for Stride8Prior {
    const STRIDE_OFFSET:usize = 7;
}
impl Stride8Prior {
    #[inline(always)]
    pub fn offset() -> usize{
        7
    }
}
impl Prior for Stride8Prior {
    fn lookup_lin(stride_byte:u8, selected_context:u8, actual_context:usize, high_nibble: Option<u8>) -> usize {
        stride_lookup_lin(stride_byte, selected_context, actual_context, high_nibble)
    }
    #[inline]
    fn which() -> WhichPrior {
      WhichPrior::STRIDE8
    }
}
*/
pub struct CMPrior {
}
impl Prior for CMPrior {
    #[allow(unused_variables)]
    fn lookup_lin(stride_byte: u8, selected_context:u8, actual_context:usize, high_nibble: Option<u8>) -> usize {
        if let Some(nibble) = high_nibble {
            (nibble as usize + 1) + 17 * actual_context
        } else {
            17 * actual_context as usize
        }
    }
    #[inline]
    fn which() -> WhichPrior {
        WhichPrior::CM
    }
}
pub struct FastCMPrior {
}
impl Prior for FastCMPrior {
    #[allow(unused_variables)]
    fn lookup_lin(stride_byte: u8, selected_context:u8, actual_context:usize, high_nibble: Option<u8>) -> usize {
        if let Some(nibble) = high_nibble {
            2 * actual_context
        } else {
            2 * actual_context + 1
        }
    }
    #[inline]
    fn which() -> WhichPrior {
        WhichPrior::FAST_CM
    }
}

pub struct SlowCMPrior {
}
impl Prior for SlowCMPrior {
    #[allow(unused_variables)]
    fn lookup_lin(stride_byte: u8, selected_context:u8, actual_context:usize, high_nibble: Option<u8>) -> usize {
        if let Some(nibble) = high_nibble {
            (nibble as usize + 1) + 17 * actual_context
        } else {
            17 * actual_context as usize
        }
    }
    #[inline]
    fn which() -> WhichPrior {
        WhichPrior::SLOW_CM
    }
}

pub struct AdvPrior {
}
impl Prior for AdvPrior {
    #[allow(unused_variables)]
    fn lookup_lin(stride_byte: u8, selected_context:u8, actual_context:usize, high_nibble: Option<u8>) -> usize {
        if let Some(nibble) = high_nibble {
            65536 + ((actual_context as usize)
                  | ((stride_byte as usize) << 8)
                  | ((nibble as usize & 0xf) << 16))
        } else {
            (actual_context as usize)
             | ((stride_byte as usize & 0xf0) << 8)
        }
    }
    fn which() -> WhichPrior {
        WhichPrior::ADV
    }
}

pub struct CDF<'a> {
    cdf:&'a mut i16x16,
}

impl<'a> CDF<'a> {
    #[inline(always)]
    pub fn cost(&self, nibble_u8:u8) -> floatX {
        let nibble = nibble_u8 as usize & 0xf;
        let mut pdf = self.cdf.extract(usize::from(nibble_u8));
        if nibble_u8 != 0 {
            pdf -= self.cdf.extract(usize::from(nibble_u8));
        }
        FastLog2u16(self.cdf.extract(15) as u16) - FastLog2u16(pdf as u16)
    }
    #[inline(always)]
    pub fn update(&mut self, nibble_u8:u8, speed: (u16, u16)) {
        assert_eq!(self.cdf.len(), 16);
        for nib_range in (nibble_u8 as usize & 0xf) .. 16 {
            self.cdf = self.cdf.replace(self.cdf.extract(nib_range) + speed.0, nib_range); // FIXME: perf: do as single op/splat
        }
        if self.cdf.extract(15) >= speed.1 {
          let CDF_BIAS = i16x16::new(1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16);
          *self.cdf = *self.cdf + CDF_BIAS - ((*self.cdf + CDF_BIAS) >> 2);
        }
    }
}

impl<'a> From<&'a mut i16x16> for CDF<'a> {
    #[inline]
    fn from(cdf: &'a mut i16x16) -> CDF<'a> {
        CDF {
            cdf:cdf,
        }
    }
}


pub fn init_cdfs(cdfs: &mut [i16x16]) {
  for item in cdfs.iter_mut() {
    *item = i16x16::new(4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60, 64);
   }
}


pub struct PriorEval<'a,
                     Alloc16x16:alloc::Allocator<i16x16>,
                     AllocU32:alloc::Allocator<u32>,
                     AllocFx8:alloc::Allocator<f32x8>,
                     > {
    input: InputPair<'a>,
    context_map: interface::PredictionModeContextMap<InputReferenceMut<'a>>,
    block_type: u8,
    local_byte_offset: usize,
    _nop: AllocU32::AllocatedMemory,    
    cm_priors: Alloc16x16::AllocatedMemory,
    slow_cm_priors: Alloc16x16::AllocatedMemory,
    fast_cm_priors: Alloc16x16::AllocatedMemory,
    stride_priors: [Alloc16x16::AllocatedMemory; 5],
    adv_priors: Alloc16x16::AllocatedMemory,
    _stride_pyramid_leaves: [u8; find_stride::NUM_LEAF_NODES],
    score: AllocFx8::AllocatedMemory,
    cm_speed: [(u16, u16);2],
    stride_speed: [(u16, u16);2],
    cur_stride: u8,
}

impl<'a,
     Alloc16x16:alloc::Allocator<i16x16>,
     AllocU32:alloc::Allocator<u32>,
     AllocFx8:alloc::Allocator<f32x8>,
     > PriorEval<'a, Alloc16x16, AllocU32, AllocFx8> {
  pub fn new(m16x16: &mut Alloc16x16,
              _m32: &mut AllocU32,
              mf: &mut AllocFx8,
              input: InputPair<'a>,
              stride: [u8; find_stride::NUM_LEAF_NODES],
              prediction_mode: interface::PredictionModeContextMap<InputReferenceMut<'a>>,
              params: &BrotliEncoderParams,
              ) -> Self {
      let do_alloc = params.prior_bitmask_detection != 0;
      let mut cm_speed = prediction_mode.context_map_speed();
      let mut stride_speed = prediction_mode.stride_context_speed();
      if cm_speed[0] == (0,0) {
          cm_speed[0] = params.literal_adaptation[2]
      }
      if cm_speed[0] == (0,0) {
          cm_speed[0] = DEFAULT_SPEED;
      }
      if cm_speed[1] == (0,0) {
          cm_speed[1] = params.literal_adaptation[3]
      }
      if cm_speed[1] == (0,0) {
          cm_speed[1] = cm_speed[0];
      }
      if stride_speed[0] == (0, 0) {
          stride_speed[0] = params.literal_adaptation[0]
      }
      if stride_speed[0] == (0, 0) {
          stride_speed[0] = DEFAULT_SPEED;
      }
      if stride_speed[1] == (0, 0) {
          stride_speed[1] = params.literal_adaptation[1]
      }
      if stride_speed[1] == (0, 0) {
          stride_speed[1] = stride_speed[0];
      }
      let mut ret = PriorEval::<Alloc16x16, AllocU32, AllocFx8>{
         input: input,
         context_map: prediction_mode,
         block_type: 0,
         cur_stride: 1,
         local_byte_offset: 0,
         _nop:  AllocU32::AllocatedMemory::default(),
         cm_priors: if do_alloc {m16x16.alloc_cell(CONTEXT_MAP_PRIOR_SIZE)} else {
             Alloc16x16::AllocatedMemory::default()},
         slow_cm_priors: if do_alloc {m16x16.alloc_cell(CONTEXT_MAP_PRIOR_SIZE)} else {
             Alloc16x16::AllocatedMemory::default()},
         fast_cm_priors: if do_alloc {m16x16.alloc_cell(CONTEXT_MAP_PRIOR_SIZE)} else {
             Alloc16x16::AllocatedMemory::default()},
         stride_priors: [
             if do_alloc {m16x16.alloc_cell(STRIDE_PRIOR_SIZE)} else {
                 Alloc16x16::AllocatedMemory::default()},
             if do_alloc {m16x16.alloc_cell(STRIDE_PRIOR_SIZE)} else {
                 Alloc16x16::AllocatedMemory::default()},
             if do_alloc {m16x16.alloc_cell(STRIDE_PRIOR_SIZE)} else {
                 Alloc16x16::AllocatedMemory::default()},
             if do_alloc {m16x16.alloc_cell(STRIDE_PRIOR_SIZE)} else {
                 Alloc16x16::AllocatedMemory::default()},
             /*if do_alloc {m16x16.alloc_cell(STRIDE_PRIOR_SIZE)} else {
                 Alloc16x16::AllocatedMemory::default()},*/],
         adv_priors: if do_alloc {m16x16.alloc_cell(ADV_PRIOR_SIZE)} else {
             Alloc16x16::AllocatedMemory::default()},
         _stride_pyramid_leaves: stride,
         score: if do_alloc {mf.alloc_cell(8192)} else {
             AllocFx8::AllocatedMemory::default()},
         cm_speed: cm_speed,
         stride_speed: stride_speed,
      };
      init_cdfs(ret.cm_priors.slice_mut());
      init_cdfs(ret.slow_cm_priors.slice_mut());
      init_cdfs(ret.fast_cm_priors.slice_mut());
      init_cdfs(ret.stride_priors[0].slice_mut());
      init_cdfs(ret.stride_priors[1].slice_mut());
      init_cdfs(ret.stride_priors[2].slice_mut());
      init_cdfs(ret.stride_priors[3].slice_mut());
      //init_cdfs(ret.stride_priors[4].slice_mut());
      init_cdfs(ret.adv_priors.slice_mut());
      ret
   }
   pub fn choose_bitmask(&mut self) {
       let epsilon = 6.0;
       let mut max_popularity = 0u32;
       let mut max_popularity_index = 0u8;
       assert_eq!(WhichPrior::NUM_PRIORS as usize, 9); // workaround rust 1.8.0 compiler bug
       let mut popularity = [0u32; 9];
       let mut bitmask = [0u8; super::interface::NUM_MIXING_VALUES];
       for (i, score) in self.score.iter().enumerate() {
           let cm_score = score.extract(WhichPrior::CM as usize);
           let slow_cm_score = score.extract(WhichPrior::SLOW_CM as usize);
           let fast_cm_score = score.extract(WhichPrior::FAST_CM as usize) + 16.0;
           let stride1_score = score.extract(WhichPrior::STRIDE1);
           let stride2_score = score.extract(WhichPrior::STRIDE2);
           let stride3_score = score.extract(WhichPrior::STRIDE3) + 16.0;
           let stride4_score = score.extract(WhichPrior::STRIDE4);
         //let stride8_score = score.extract(WhichPrior::STRIDE8) * 1.125 + 16.0;
           let stride8_score = stride4_score + 1; // FIXME: never lowest -- ignore stride 8
           let stride_score = core::cmp::min(stride1_score as u64,
                                             core::cmp::min(stride2_score as u64,
                                                            core::cmp::min(stride3_score as u64,
                                                                           core::cmp::min(stride4_score as u64,
                                                                                          stride8_score as u64))));
                                  
           let adv_score = score.extract(WhichPrior::ADV);
           if adv_score + epsilon < stride_score as floatX && adv_score + epsilon < cm_score && adv_score + epsilon < slow_cm_score && adv_score + epsilon < fast_cm_score {
               bitmask[i] = 1;
           } else if slow_cm_score + epsilon < stride_score as floatX && slow_cm_score + epsilon < cm_score && slow_cm_score + epsilon < fast_cm_score {
               bitmask[i] = 2;
           } else if fast_cm_score + epsilon < stride_score as floatX && fast_cm_score + epsilon < cm_score {
               bitmask[i] = 3;
           } else if epsilon + (stride_score as floatX) < cm_score {
               bitmask[i] = WhichPrior::STRIDE1 as u8;
               if stride_score == stride8_score as u64 {
                   //bitmask[i] = WhichPrior::STRIDE8 as u8;
               }
               if stride_score == stride4_score as u64 {
                   bitmask[i] = WhichPrior::STRIDE4 as u8;
               }
               if stride_score == stride3_score as u64 {
                   bitmask[i] = WhichPrior::STRIDE3 as u8;
               }
               if stride_score == stride2_score as u64 {
                   bitmask[i] = WhichPrior::STRIDE2 as u8;
               }
               if stride_score == stride1_score as u64 {
                   bitmask[i] = WhichPrior::STRIDE1 as u8;
               }
           } else {
               bitmask[i] = 0;
           }
           if stride_score == 0 {
               bitmask[i] = max_popularity_index;
               //eprintln!("Miss {}[{}] ~ {}", bitmask[i], i, max_popularity_index);
           } else {
               popularity[bitmask[i] as usize] += 1;
               if popularity[bitmask[i] as usize] > max_popularity {
                   max_popularity = popularity[bitmask[i] as usize];
                   max_popularity_index = bitmask[i];
               }
               //eprintln!("Score {} {} {} {} {}: {}[{}] max={},{}", cm_score, adv_score, slow_cm_score, fast_cm_score, stride_score, bitmask[i], i, max_popularity, max_popularity_index);
           }
       }
       self.context_map.set_mixing_values(&bitmask);
   }
   pub fn free(&mut self,
               m16x16: &mut Alloc16x16,
               _m32: &mut AllocU32,
               mf8: &mut AllocFx8) {
       mf8.free_cell(core::mem::replace(&mut self.score, AllocFx8::AllocatedMemory::default()));
       m16x16.free_cell(core::mem::replace(&mut self.cm_priors, Alloc16x16::AllocatedMemory::default()));
       m16x16.free_cell(core::mem::replace(&mut self.slow_cm_priors, Alloc16x16::AllocatedMemory::default()));
       m16x16.free_cell(core::mem::replace(&mut self.fast_cm_priors, Alloc16x16::AllocatedMemory::default()));
       m16x16.free_cell(core::mem::replace(&mut self.stride_priors[0], Alloc16x16::AllocatedMemory::default()));
       m16x16.free_cell(core::mem::replace(&mut self.stride_priors[1], Alloc16x16::AllocatedMemory::default()));
       m16x16.free_cell(core::mem::replace(&mut self.stride_priors[2], Alloc16x16::AllocatedMemory::default()));
       m16x16.free_cell(core::mem::replace(&mut self.stride_priors[3], Alloc16x16::AllocatedMemory::default()));
       m16x16.free_cell(core::mem::replace(&mut self.stride_priors[4], Alloc16x16::AllocatedMemory::default()));
       m16x16.free_cell(core::mem::replace(&mut self.adv_priors, Alloc16x16::AllocatedMemory::default()));
   }
                
   pub fn take_prediction_mode(&mut self) -> interface::PredictionModeContextMap<InputReferenceMut<'a>> {
       core::mem::replace(&mut self.context_map, interface::PredictionModeContextMap::<InputReferenceMut<'a>>{
          literal_context_map:InputReferenceMut::default(),
          predmode_speed_and_distance_context_map:InputReferenceMut::default(),
       })
   }
  fn update_cost_base(&mut self, stride_prior: [u8;8], stride_prior_offset:usize, selected_bits: u8, cm_prior: usize, literal: u8) {
      let mut l_score = f32x8::splat(0.0);
      let mut h_score = f32x8::splat(0.0);
      let base_stride_prior = stride_prior[stride_prior_offset.wrapping_sub(self.cur_stride as usize) & 7];
      let hscore_index = upper_score_index(base_stride_prior, selected_bits, cm_prior);
      let lscore_index = upper_score_index(base_stride_prior, selected_bits, cm_prior, literal);
       {
           type CurPrior = CMPrior;
           let mut cdf = CurPrior::lookup_mut(self.cm_priors.slice_mut(),
                                              base_stride_prior, selected_bits, cm_prior, None);
           h_score = h_score.replace(cdf.cost(literal>>4), CurPrior::which());
           cdf.update(literal >> 4, self.cm_speed[1]);
       }
       {
           type CurPrior = CMPrior;
           let mut cdf = CurPrior::lookup_mut(self.cm_priors.slice_mut(),
                                              base_stride_prior, selected_bits, cm_prior, Some(literal >> 4));
           l_score = l_score.replace(cdf.cost(literal&0xf), CurPrior::which());
           cdf.update(literal&0xf, self.cm_speed[0]);
       }
       {
           type CurPrior = SlowCMPrior;
           let mut cdf = CurPrior::lookup_mut(self.slow_cm_priors.slice_mut(),
                                              base_stride_prior, selected_bits, cm_prior, None);
           h_score = h_score.replace(cdf.cost(literal>>4), CurPrior::which());
           cdf.update(literal >> 4, (0,1024));
       }
       {
           type CurPrior = SlowCMPrior;
           let mut cdf = CurPrior::lookup_mut(self.slow_cm_priors.slice_mut(),
                                              base_stride_prior, selected_bits, cm_prior, Some(literal >> 4));
           l_score = l_score.replace(cdf.cost(literal&0xf), CurPrior::which());
           cdf.update(literal&0xf, (0,1024));
       }
       {
           type CurPrior = FastCMPrior;
           let mut cdf = CurPrior::lookup_mut(self.fast_cm_priors.slice_mut(),
                                              base_stride_prior, selected_bits, cm_prior, None);
           h_score = h_score.replace(cdf.cost(literal>>4), CurPrior::which());
           cdf.update(literal >> 4, self.cm_speed[0]);
       }
       {
           type CurPrior = FastCMPrior;
           let mut cdf = CurPrior::lookup_mut(self.fast_cm_priors.slice_mut(),
                                              base_stride_prior, selected_bits, cm_prior, Some(literal >> 4));
           l_score = l_score.replace(cdf.cost(literal&0xf), CurPrior::which());
           cdf.update(literal&0xf, self.cm_speed[0]);
       }
       {
           type CurPrior = Stride1Prior;
           let mut cdf = CurPrior::lookup_mut(self.stride_priors[0].slice_mut(),
                                              stride_prior[stride_prior_offset.wrapping_sub(CurPrior::offset())&7], selected_bits, cm_prior, None);
           h_score = h_score.replace(cdf.cost(literal>>4), CurPrior::which());
           cdf.update(literal >> 4, self.stride_speed[1]);
       }
       {
           type CurPrior = Stride1Prior;
           let mut cdf = CurPrior::lookup_mut(self.stride_priors[0].slice_mut(),
                                              stride_prior[stride_prior_offset.wrapping_sub(CurPrior::offset())&7],
                                              selected_bits,
                                              cm_prior,
                                              Some(literal >> 4));
           l_score = l_score.replace(cdf.cost(literal&0xf), CurPrior::which());
           cdf.update(literal&0xf, self.stride_speed[0]);
       }
       {
           type CurPrior = Stride2Prior;
           let mut cdf = CurPrior::lookup_mut(self.stride_priors[1].slice_mut(),
                                              stride_prior[stride_prior_offset.wrapping_sub(CurPrior::offset())&7], selected_bits, cm_prior, None);
           h_score = h_score.replace(cdf.cost(literal>>4), CurPrior::which());
           cdf.update(literal >> 4, self.stride_speed[1]);
       }
       {
           type CurPrior = Stride2Prior;
           let mut cdf = CurPrior::lookup_mut(self.stride_priors[1].slice_mut(),
                                              stride_prior[stride_prior_offset.wrapping_sub(CurPrior::offset())&7],
                                              selected_bits,
                                              cm_prior,
                                              Some(literal >> 4));
           l_score = l_score.replace(cdf.cost(literal&0xf), CurPrior::which());
           cdf.update(literal&0xf, self.stride_speed[0]);
       }
       {
           type CurPrior = Stride3Prior;
           let mut cdf = CurPrior::lookup_mut(self.stride_priors[2].slice_mut(),
                                              stride_prior[stride_prior_offset.wrapping_sub(CurPrior::offset())&7], selected_bits, cm_prior, None);
           h_score = h_score.replace(cdf.cost(literal>>4), CurPrior::which());
           cdf.update(literal >> 4, self.stride_speed[1]);
       }
       {
           type CurPrior = Stride3Prior;
           let score_index = CurPrior::score_index(base_stride_prior, selected_bits, cm_prior, Some(literal >> 4));
           let mut cdf = CurPrior::lookup_mut(self.stride_priors[2].slice_mut(),
                                              stride_prior[stride_prior_offset.wrapping_sub(CurPrior::offset())&7],
                                              selected_bits,
                                              cm_prior,
                                              Some(literal >> 4));
           l_score = l_score.replace(cdf.cost(literal&0xf), CurPrior::which());
           cdf.update(literal&0xf, self.stride_speed[0]);
       }
       {
           type CurPrior = Stride4Prior;
           let mut cdf = CurPrior::lookup_mut(self.stride_priors[3].slice_mut(),
                                              stride_prior[stride_prior_offset.wrapping_sub(CurPrior::offset())&7], selected_bits, cm_prior, None);
           h_score = h_score.replace(cdf.cost(literal>>4), CurPrior::which());
           cdf.update(literal >> 4, self.stride_speed[1]);
       }
       {
           type CurPrior = Stride4Prior;
           let mut cdf = CurPrior::lookup_mut(self.stride_priors[3].slice_mut(),
                                              stride_prior[stride_prior_offset.wrapping_sub(CurPrior::offset())&7],
                                              selected_bits,
                                              cm_prior,
                                              Some(literal >> 4));
           l_score = l_score.replace(cdf.cost(literal&0xf), CurPrior::which());
           cdf.update(literal&0xf, self.stride_speed[0]);
       }
/*       {
           type CurPrior = Stride8Prior;
           let score_index = CurPrior::score_index(base_stride_prior, selected_bits, cm_prior, None);
           let mut cdf = CurPrior::lookup_mut(self.stride_priors[4].slice_mut(),
                                              stride_prior[stride_prior_offset.wrapping_sub(CurPrior::offset())&7], selected_bits, cm_prior, None);
           h_score = h_score.replace(cdf.cost(literal>>4), CurPrior::which());
           cdf.update(literal >> 4, self.stride_speed[1]);
       }
       {
           type CurPrior = Stride8Prior;
           let mut cdf = CurPrior::lookup_mut(self.stride_priors[4].slice_mut(),
                                              stride_prior[stride_prior_offset.wrapping_sub(CurPrior::offset()) & 7],
                                              selected_bits,
                                              cm_prior,
                                              Some(literal >> 4));
           l_score = l_score.replace(cdf.cost(literal&0xf), CurPrior::which());
           cdf.update(literal&0xf, self.stride_speed[0]);
       }
*/
       type CurPrior = AdvPrior;
       {
           let score_index = CurPrior::score_index(base_stride_prior, selected_bits, cm_prior, None);
           let mut cdf = CurPrior::lookup_mut(self.adv_priors.slice_mut(),
                                              base_stride_prior, selected_bits, cm_prior, None);
           h_score = h_score.replace(cdf.cost(literal>>4), CurPrior::which());
           cdf.update(literal >> 4, self.stride_speed[1]);
       }
       {
           let score_index = CurPrior::score_index(base_stride_prior, selected_bits, cm_prior, Some(literal >> 4));
           let mut cdf = CurPrior::lookup_mut(self.adv_priors.slice_mut(),
                                              base_stride_prior, selected_bits, cm_prior, Some(literal >> 4));
           l_score = l_score.replace(cdf.cost(literal&0xf), CurPrior::which());
           cdf.update(literal&0xf, self.stride_speed[0]);
       }
       self.score[lscore_index] += l_score;
       self.score[hscore_index] += h_score;
  }
}
impl<'a, Alloc16x16: alloc::Allocator<i16x16>,
     AllocU32:alloc::Allocator<u32>,
     AllocFx8: alloc::Allocator<f32x8>> IRInterpreter for PriorEval<'a, Alloc16x16, AllocU32, AllocFx8> {
    #[inline]
    fn inc_local_byte_offset(&mut self, inc: usize) {
        self.local_byte_offset += inc;
    }
    #[inline]
    fn local_byte_offset(&self) -> usize {
        self.local_byte_offset
    }
    #[inline]
    fn update_block_type(&mut self, new_type: u8, stride: u8) {
        self.block_type = new_type;
        self.cur_stride = stride;
    }
    #[inline]
    fn block_type(&self) -> u8 {
        self.block_type
    }
    #[inline]
    fn literal_data_at_offset(&self, index:usize) -> u8 {
        self.input[index]
    }
    #[inline]
    fn literal_context_map(&self) -> &[u8] {
        self.context_map.literal_context_map.slice()
    }
    #[inline]
    fn prediction_mode(&self) -> ::interface::LiteralPredictionModeNibble {
        self.context_map.literal_prediction_mode()
    }
    #[inline]
    fn update_cost(&mut self, stride_prior: [u8;8], stride_prior_offset: usize, selected_bits: u8, cm_prior: usize, literal: u8) {
        //let stride = self.cur_stride as usize;
        self.update_cost_base(stride_prior, stride_prior_offset, selected_bits, cm_prior, literal)
    }
}



impl<'a, 'b, Alloc16x16: alloc::Allocator<i16x16>,
     AllocU32:alloc::Allocator<u32>,
     AllocFx8: alloc::Allocator<f32x8>>  interface::CommandProcessor<'b> for PriorEval<'a, Alloc16x16, AllocU32, AllocFx8> {
    #[inline]
    fn push(&mut self,
            val: interface::Command<InputReference<'b>>) {
        push_base(self, val)
    }
}

