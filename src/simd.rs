//! Eight-lane AVX2 types (from fraktal256-rs): the CPU renderer shades pixels
//! eight at a time ("octets"): `F8` (f32 x 8), `I8` (i32 x 8), `M8` (a lane
//! mask) and `V8` (a 3-vector of `F8`). The crate is built for x86-64-v3
//! (.cargo/config.toml) and main() checks the CPU before any of it runs, so
//! every intrinsic here is sound to call.
#![allow(unused_unsafe)]

use crate::math::Vec3;
use core::arch::x86_64::*;
use core::ops::*;

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct F8(pub __m256);

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct I8(pub __m256i);

/// A lane mask: all bits set in a lane = true.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct M8(pub __m256);

impl core::fmt::Debug for F8 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let a = self.to_array();
        let mut l = f.debug_list();
        for v in a {
            l.entry(&((v * 1000.0) as i32));
        }
        l.finish()
    }
}

// ---- F8 -------------------------------------------------------------------

impl F8 {
    pub const LANES: usize = 8;

    #[inline(always)]
    pub fn splat(v: f32) -> F8 {
        unsafe { F8(_mm256_set1_ps(v)) }
    }
    #[inline(always)]
    pub fn zero() -> F8 {
        unsafe { F8(_mm256_setzero_ps()) }
    }
    #[inline(always)]
    pub fn one() -> F8 {
        F8::splat(1.0)
    }
    /// 0, 1, 2, ... 7
    #[inline(always)]
    pub fn iota() -> F8 {
        unsafe { F8(_mm256_setr_ps(0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0)) }
    }
    #[inline(always)]
    pub fn from_array(a: [f32; 8]) -> F8 {
        unsafe { F8(unsafe { _mm256_loadu_ps(a.as_ptr()) }) }
    }
    #[inline(always)]
    pub fn to_array(self) -> [f32; 8] {
        unsafe {
            let mut a = [0.0f32; 8];
            unsafe { _mm256_storeu_ps(a.as_mut_ptr(), self.0) };
            a
        }
    }
    #[inline(always)]
    pub fn lane(self, i: usize) -> f32 {
        self.to_array()[i & 7]
    }
    /// The first eight values of `s`.
    #[inline(always)]
    pub fn load(s: &[f32]) -> F8 {
        unsafe {
            assert!(s.len() >= 8);
            F8(unsafe { _mm256_loadu_ps(s.as_ptr()) })
        }
    }
    /// Into the first eight values of `d`.
    #[inline(always)]
    pub fn store(self, d: &mut [f32]) {
        unsafe {
            assert!(d.len() >= 8);
            unsafe { _mm256_storeu_ps(d.as_mut_ptr(), self.0) }
        }
    }
    /// Gathers `s[idx]` per lane; indices are clamped into the slice.
    #[inline(always)]
    pub fn gather(s: &[f32], idx: I8) -> F8 {
        unsafe {
            assert!(!s.is_empty() && s.len() <= i32::MAX as usize);
            let i = _mm256_min_epu32(idx.0, _mm256_set1_epi32(s.len() as i32 - 1));
            F8(unsafe { _mm256_i32gather_ps::<4>(s.as_ptr(), i) })
        }
    }

    /// `gather` for indices known to be inside the slice (checked in debug
    /// builds only): the heightmap fetches, whose indices are clamped.
    #[inline(always)]
    pub fn gather_in_bounds(s: &[f32], idx: I8) -> F8 {
        debug_assert!(idx.to_array().iter().all(|&i| (i as usize) < s.len()));
        unsafe { F8(_mm256_i32gather_ps::<4>(s.as_ptr(), idx.0)) }
    }

    /// self * b + c (one rounding)
    #[inline(always)]
    pub fn mul_add(self, b: F8, c: F8) -> F8 {
        unsafe { F8(_mm256_fmadd_ps(self.0, b.0, c.0)) }
    }
    /// c - self * b (one rounding)
    #[inline(always)]
    pub fn neg_mul_add(self, b: F8, c: F8) -> F8 {
        unsafe { F8(_mm256_fnmadd_ps(self.0, b.0, c.0)) }
    }
    #[inline(always)]
    pub fn min(self, b: F8) -> F8 {
        unsafe { F8(_mm256_min_ps(self.0, b.0)) }
    }
    #[inline(always)]
    pub fn max(self, b: F8) -> F8 {
        unsafe { F8(_mm256_max_ps(self.0, b.0)) }
    }
    #[inline(always)]
    pub fn clamp(self, lo: f32, hi: f32) -> F8 {
        self.max(F8::splat(lo)).min(F8::splat(hi))
    }
    #[inline(always)]
    pub fn saturate(self) -> F8 {
        self.clamp(0.0, 1.0)
    }
    #[inline(always)]
    pub fn abs(self) -> F8 {
        unsafe { F8(_mm256_andnot_ps(_mm256_set1_ps(-0.0), self.0)) }
    }
    /// The sign bit of `s` on the magnitude of self.
    #[inline(always)]
    pub fn copysign(self, s: F8) -> F8 {
        unsafe {
            let sign = _mm256_set1_ps(-0.0);
            F8(_mm256_or_ps(_mm256_andnot_ps(sign, self.0), _mm256_and_ps(sign, s.0)))
        }
    }
    #[inline(always)]
    pub fn sqrt(self) -> F8 {
        unsafe { F8(_mm256_sqrt_ps(self.0)) }
    }
    /// 1 / sqrt, hardware estimate (about 12 bits)
    #[inline(always)]
    pub fn rsqrt_est(self) -> F8 {
        unsafe { F8(_mm256_rsqrt_ps(self.0)) }
    }
    /// 1 / sqrt, estimate plus one Newton step (about 22 bits)
    #[inline(always)]
    pub fn rsqrt(self) -> F8 {
        let y = self.rsqrt_est();
        y * (F8::splat(1.5) - F8::splat(0.5) * self * y * y)
    }
    /// 1 / x, hardware estimate (about 12 bits)
    #[inline(always)]
    pub fn recip_est(self) -> F8 {
        unsafe { F8(_mm256_rcp_ps(self.0)) }
    }
    /// 1 / x, estimate plus one Newton step
    #[inline(always)]
    pub fn recip(self) -> F8 {
        let y = self.recip_est();
        y * (F8::splat(2.0) - self * y)
    }
    #[inline(always)]
    pub fn floor(self) -> F8 {
        unsafe { F8(_mm256_round_ps::<{ _MM_FROUND_TO_NEG_INF | _MM_FROUND_NO_EXC }>(self.0)) }
    }
    #[inline(always)]
    pub fn ceil(self) -> F8 {
        unsafe { F8(_mm256_round_ps::<{ _MM_FROUND_TO_POS_INF | _MM_FROUND_NO_EXC }>(self.0)) }
    }
    /// Round half to even.
    #[inline(always)]
    pub fn round(self) -> F8 {
        unsafe { F8(_mm256_round_ps::<{ _MM_FROUND_TO_NEAREST_INT | _MM_FROUND_NO_EXC }>(self.0)) }
    }
    #[inline(always)]
    pub fn trunc(self) -> F8 {
        unsafe { F8(_mm256_round_ps::<{ _MM_FROUND_TO_ZERO | _MM_FROUND_NO_EXC }>(self.0)) }
    }
    /// x - floor(x)
    #[inline(always)]
    pub fn fract(self) -> F8 {
        self - self.floor()
    }
    #[inline(always)]
    pub fn lerp(a: F8, b: F8, t: F8) -> F8 {
        (b - a).mul_add(t, a)
    }
    /// Hermite smoothstep of x between edges a and b.
    #[inline(always)]
    pub fn smoothstep(a: f32, b: f32, x: F8) -> F8 {
        let t = ((x - a) * (1.0 / (b - a))).saturate();
        t * t * (F8::splat(3.0) - t - t)
    }

    #[inline(always)]
    pub fn lt(self, b: F8) -> M8 {
        unsafe { M8(_mm256_cmp_ps::<_CMP_LT_OQ>(self.0, b.0)) }
    }
    #[inline(always)]
    pub fn le(self, b: F8) -> M8 {
        unsafe { M8(_mm256_cmp_ps::<_CMP_LE_OQ>(self.0, b.0)) }
    }
    #[inline(always)]
    pub fn gt(self, b: F8) -> M8 {
        unsafe { M8(_mm256_cmp_ps::<_CMP_GT_OQ>(self.0, b.0)) }
    }
    #[inline(always)]
    pub fn ge(self, b: F8) -> M8 {
        unsafe { M8(_mm256_cmp_ps::<_CMP_GE_OQ>(self.0, b.0)) }
    }
    #[inline(always)]
    pub fn eq(self, b: F8) -> M8 {
        unsafe { M8(_mm256_cmp_ps::<_CMP_EQ_OQ>(self.0, b.0)) }
    }
    #[inline(always)]
    pub fn ne(self, b: F8) -> M8 {
        unsafe { M8(_mm256_cmp_ps::<_CMP_NEQ_UQ>(self.0, b.0)) }
    }
    /// Lanes that are NaN.
    #[inline(always)]
    pub fn is_nan(self) -> M8 {
        unsafe { M8(_mm256_cmp_ps::<_CMP_UNORD_Q>(self.0, self.0)) }
    }

    /// `t` where the mask is set, else `f`.
    #[inline(always)]
    pub fn select(m: M8, t: F8, f: F8) -> F8 {
        unsafe { F8(_mm256_blendv_ps(f.0, t.0, m.0)) }
    }
    /// self where the mask is set, else 0.
    #[inline(always)]
    pub fn and(self, m: M8) -> F8 {
        unsafe { F8(_mm256_and_ps(self.0, m.0)) }
    }

    #[inline(always)]
    pub fn hmin(self) -> f32 {
        self.to_array().into_iter().fold(f32::INFINITY, f32::min)
    }
    #[inline(always)]
    pub fn hmax(self) -> f32 {
        self.to_array().into_iter().fold(f32::NEG_INFINITY, f32::max)
    }
    #[inline(always)]
    pub fn hsum(self) -> f32 {
        self.to_array().into_iter().sum()
    }

    /// Truncate toward zero.
    #[inline(always)]
    pub fn to_i8(self) -> I8 {
        unsafe { I8(_mm256_cvttps_epi32(self.0)) }
    }
    /// Round to nearest.
    #[inline(always)]
    pub fn round_i8(self) -> I8 {
        unsafe { I8(_mm256_cvtps_epi32(self.0)) }
    }
    /// The bits, unchanged.
    #[inline(always)]
    pub fn to_bits(self) -> I8 {
        unsafe { I8(_mm256_castps_si256(self.0)) }
    }

    /// sin(x): range reduced to [-pi, pi], folded into [-pi/2, pi/2], degree-9
    /// odd polynomial (about 2e-7 absolute error).
    #[inline(always)]
    pub fn sin(self) -> F8 {
        use core::f32::consts::{FRAC_1_PI, PI};
        let k = (self * (0.5 * FRAC_1_PI)).round();
        let x = k.neg_mul_add(F8::splat(2.0 * PI), self);
        let a = x.abs();
        let a = a.min(F8::splat(PI) - a).copysign(x);
        let x2 = a * a;
        let p = F8::splat(2.601_903e-6).mul_add(x2, F8::splat(-1.980_741e-4));
        let p = p.mul_add(x2, F8::splat(8.333_025e-3));
        let p = p.mul_add(x2, F8::splat(-1.666_665_7e-1));
        (p * x2).mul_add(a, a)
    }
    #[inline(always)]
    pub fn cos(self) -> F8 {
        (self + core::f32::consts::FRAC_PI_2).sin()
    }
    /// 2^x for x in [-126, 126]: round to the nearest integer, then a degree-6
    /// Taylor polynomial on the fraction in [-0.5, 0.5] (about 3e-7 relative).
    #[inline(always)]
    // (0.693_147_2 is the polynomial's fitted coefficient, near ln 2 by nature)
    #[allow(clippy::approx_constant)]
    pub fn exp2(self) -> F8 {
        unsafe {
            let x = self.clamp(-126.0, 126.0);
            let i = x.round();
            let f = x - i;
            let p = F8::splat(1.540_353e-4).mul_add(f, F8::splat(1.333_355_8e-3));
            let p = p.mul_add(f, F8::splat(9.618_129e-3)).mul_add(f, F8::splat(5.550_411e-2));
            let p = p.mul_add(f, F8::splat(0.240_226_5)).mul_add(f, F8::splat(0.693_147_2)).mul_add(f, F8::one());
            let scale = I8(_mm256_slli_epi32::<23>(_mm256_add_epi32(i.round_i8().0, _mm256_set1_epi32(127))));
            p * scale.cast_f8()
        }
    }
    /// log2(x) for normal positive x: exponent plus an atanh series on the
    /// mantissa in [sqrt(1/2), sqrt(2)) (about 1e-7 absolute).
    #[inline(always)]
    pub fn log2(self) -> F8 {
        let bits = self.to_bits();
        let e = (bits.shr::<23>() - 127).to_f8();
        let m = (bits & 0x007f_ffff | 0x3f80_0000).cast_f8();
        let big = m.gt(F8::splat(core::f32::consts::SQRT_2));
        let m = F8::select(big, m * 0.5, m);
        let e = e + F8::one().and(big);
        let t = (m - 1.0) / (m + 1.0);
        let t2 = t * t;
        let p = F8::splat(2.0 / 9.0).mul_add(t2, F8::splat(2.0 / 7.0));
        let p = p.mul_add(t2, F8::splat(2.0 / 5.0)).mul_add(t2, F8::splat(2.0 / 3.0)).mul_add(t2, F8::splat(2.0));
        (p * t).mul_add(F8::splat(core::f32::consts::LOG2_E), e)
    }
    #[inline(always)]
    pub fn ln(self) -> F8 {
        self.log2() * core::f32::consts::LN_2
    }
    /// self^e for self > 0.
    #[inline(always)]
    pub fn powf(self, e: F8) -> F8 {
        (self.log2() * e).exp2()
    }
    /// atan(x): reduced to [0, 1], odd minimax polynomial (about 1e-5 rad).
    #[inline(always)]
    pub fn atan(self) -> F8 {
        let a = self.abs();
        let inv = a.gt(F8::one());
        let t = F8::select(inv, F8::one() / a, a);
        let r = Self::atan01(t);
        let r = F8::select(inv, F8::splat(core::f32::consts::FRAC_PI_2) - r, r);
        r.copysign(self)
    }
    #[inline(always)]
    fn atan01(t: F8) -> F8 {
        let t2 = t * t;
        let p = F8::splat(-0.011_721_20).mul_add(t2, F8::splat(0.052_653_32));
        let p = p.mul_add(t2, F8::splat(-0.116_432_87)).mul_add(t2, F8::splat(0.193_543_46));
        let p = p.mul_add(t2, F8::splat(-0.332_623_47)).mul_add(t2, F8::splat(0.999_977_26));
        p * t
    }
    /// atan2(self = y, x) in [-pi, pi].
    #[inline(always)]
    pub fn atan2(self, x: F8) -> F8 {
        use core::f32::consts::{FRAC_PI_2, PI};
        let (ay, ax) = (self.abs(), x.abs());
        let swap = ay.gt(ax);
        let num = ax.min(ay);
        let den = ax.max(ay).max(F8::splat(1.0e-30));
        let r = Self::atan01(num / den);
        let r = F8::select(swap, F8::splat(FRAC_PI_2) - r, r);
        let r = F8::select(x.lt(F8::zero()), F8::splat(PI) - r, r);
        r.copysign(self)
    }
    #[inline(always)]
    pub fn exp(self) -> F8 {
        (self * core::f32::consts::LOG2_E).exp2()
    }
    /// vhaddps: in each 128-bit half, [a0+a1, a2+a3, b0+b1, b2+b3].
    #[inline(always)]
    pub fn hadd(self, b: F8) -> F8 {
        unsafe { F8(_mm256_hadd_ps(self.0, b.0)) }
    }
    /// vpermps: lane i of the result is `self[idx[i] & 7]`.
    #[inline(always)]
    pub fn permute(self, idx: I8) -> F8 {
        unsafe { F8(_mm256_permutevar8x32_ps(self.0, idx.0)) }
    }
}

macro_rules! f8_ops {
    ($tr:ident, $f:ident, $tra:ident, $fa:ident, $intr:ident) => {
        impl $tr for F8 {
            type Output = F8;
            #[inline(always)]
            fn $f(self, b: F8) -> F8 {
                F8(unsafe { $intr(self.0, b.0) })
            }
        }
        impl $tr<f32> for F8 {
            type Output = F8;
            #[inline(always)]
            fn $f(self, b: f32) -> F8 {
                unsafe { F8(unsafe { $intr(self.0, _mm256_set1_ps(b)) }) }
            }
        }
        impl $tr<F8> for f32 {
            type Output = F8;
            #[inline(always)]
            fn $f(self, b: F8) -> F8 {
                unsafe { F8(unsafe { $intr(_mm256_set1_ps(self), b.0) }) }
            }
        }
        impl $tra for F8 {
            #[inline(always)]
            fn $fa(&mut self, b: F8) {
                self.0 = unsafe { $intr(self.0, b.0) };
            }
        }
        impl $tra<f32> for F8 {
            #[inline(always)]
            fn $fa(&mut self, b: f32) {
                unsafe {
                    self.0 = unsafe { $intr(self.0, _mm256_set1_ps(b)) };
                }
            }
        }
    };
}
f8_ops!(Add, add, AddAssign, add_assign, _mm256_add_ps);
f8_ops!(Sub, sub, SubAssign, sub_assign, _mm256_sub_ps);
f8_ops!(Mul, mul, MulAssign, mul_assign, _mm256_mul_ps);
f8_ops!(Div, div, DivAssign, div_assign, _mm256_div_ps);

impl Neg for F8 {
    type Output = F8;
    #[inline(always)]
    fn neg(self) -> F8 {
        unsafe { F8(_mm256_xor_ps(self.0, _mm256_set1_ps(-0.0))) }
    }
}

// ---- M8 -------------------------------------------------------------------

impl M8 {
    #[inline(always)]
    pub fn all_true() -> M8 {
        unsafe { M8(_mm256_castsi256_ps(_mm256_set1_epi32(-1))) }
    }
    #[inline(always)]
    pub fn none_set() -> M8 {
        unsafe { M8(_mm256_setzero_ps()) }
    }
    /// One bit per lane (lane 0 = bit 0).
    #[inline(always)]
    pub fn bits(self) -> u32 {
        unsafe { _mm256_movemask_ps(self.0) as u32 }
    }
    #[inline(always)]
    pub fn from_bits(b: u32) -> M8 {
        unsafe {
            let lanes = _mm256_setr_epi32(1, 2, 4, 8, 16, 32, 64, 128);
            let v = _mm256_and_si256(_mm256_set1_epi32(b as i32), lanes);
            M8(_mm256_castsi256_ps(_mm256_cmpeq_epi32(v, lanes)))
        }
    }
    #[inline(always)]
    pub fn any(self) -> bool {
        self.bits() != 0
    }
    #[inline(always)]
    pub fn all(self) -> bool {
        self.bits() == 0xFF
    }
    #[inline(always)]
    pub fn none(self) -> bool {
        self.bits() == 0
    }
    #[inline(always)]
    pub fn lane(self, i: usize) -> bool {
        self.bits() >> (i & 7) & 1 != 0
    }
    /// self and not b
    #[inline(always)]
    pub fn and_not(self, b: M8) -> M8 {
        unsafe { M8(_mm256_andnot_ps(b.0, self.0)) }
    }
    /// 1.0 where set, else 0.0
    #[inline(always)]
    pub fn to_f8(self) -> F8 {
        F8::one().and(self)
    }
    #[inline(always)]
    pub fn to_i8(self) -> I8 {
        unsafe { I8(_mm256_castps_si256(self.0)) }
    }
}

impl BitAnd for M8 {
    type Output = M8;
    #[inline(always)]
    fn bitand(self, b: M8) -> M8 {
        unsafe { M8(_mm256_and_ps(self.0, b.0)) }
    }
}
impl BitOr for M8 {
    type Output = M8;
    #[inline(always)]
    fn bitor(self, b: M8) -> M8 {
        unsafe { M8(_mm256_or_ps(self.0, b.0)) }
    }
}
impl BitXor for M8 {
    type Output = M8;
    #[inline(always)]
    fn bitxor(self, b: M8) -> M8 {
        unsafe { M8(_mm256_xor_ps(self.0, b.0)) }
    }
}
impl Not for M8 {
    type Output = M8;
    #[inline(always)]
    fn not(self) -> M8 {
        self ^ M8::all_true()
    }
}
impl BitAndAssign for M8 {
    #[inline(always)]
    fn bitand_assign(&mut self, b: M8) {
        *self = *self & b;
    }
}
impl BitOrAssign for M8 {
    #[inline(always)]
    fn bitor_assign(&mut self, b: M8) {
        *self = *self | b;
    }
}

// ---- I8 -------------------------------------------------------------------

impl I8 {
    #[inline(always)]
    pub fn splat(v: i32) -> I8 {
        unsafe { I8(_mm256_set1_epi32(v)) }
    }
    #[inline(always)]
    pub fn zero() -> I8 {
        unsafe { I8(_mm256_setzero_si256()) }
    }
    /// 0, 1, 2, ... 7
    #[inline(always)]
    pub fn iota() -> I8 {
        unsafe { I8(_mm256_setr_epi32(0, 1, 2, 3, 4, 5, 6, 7)) }
    }
    #[inline(always)]
    pub fn from_array(a: [i32; 8]) -> I8 {
        unsafe { I8(unsafe { _mm256_loadu_si256(a.as_ptr() as *const __m256i) }) }
    }
    #[inline(always)]
    pub fn to_array(self) -> [i32; 8] {
        unsafe {
            let mut a = [0i32; 8];
            unsafe { _mm256_storeu_si256(a.as_mut_ptr() as *mut __m256i, self.0) };
            a
        }
    }
    #[inline(always)]
    pub fn lane(self, i: usize) -> i32 {
        self.to_array()[i & 7]
    }
    /// Gathers `s[idx]` per lane; indices are clamped into the slice.
    #[inline(always)]
    pub fn gather(s: &[i32], idx: I8) -> I8 {
        unsafe {
            assert!(!s.is_empty() && s.len() <= i32::MAX as usize);
            let i = _mm256_min_epu32(idx.0, _mm256_set1_epi32(s.len() as i32 - 1));
            I8(unsafe { _mm256_i32gather_epi32::<4>(s.as_ptr(), i) })
        }
    }
    #[inline(always)]
    pub fn to_f8(self) -> F8 {
        unsafe { F8(_mm256_cvtepi32_ps(self.0)) }
    }
    /// The bits, unchanged.
    #[inline(always)]
    pub fn cast_f8(self) -> F8 {
        unsafe { F8(_mm256_castsi256_ps(self.0)) }
    }
    /// Wrapping low 32 bits of the product.
    #[inline(always)]
    pub fn mul_lo(self, b: I8) -> I8 {
        unsafe { I8(_mm256_mullo_epi32(self.0, b.0)) }
    }
    #[inline(always)]
    pub fn min(self, b: I8) -> I8 {
        unsafe { I8(_mm256_min_epi32(self.0, b.0)) }
    }
    #[inline(always)]
    pub fn max(self, b: I8) -> I8 {
        unsafe { I8(_mm256_max_epi32(self.0, b.0)) }
    }
    /// Logical shift right.
    #[inline(always)]
    pub fn shr<const N: i32>(self) -> I8 {
        unsafe { I8(_mm256_srli_epi32::<N>(self.0)) }
    }
    /// Arithmetic shift right.
    #[inline(always)]
    pub fn sar<const N: i32>(self) -> I8 {
        unsafe { I8(_mm256_srai_epi32::<N>(self.0)) }
    }
    #[inline(always)]
    pub fn shl<const N: i32>(self) -> I8 {
        unsafe { I8(_mm256_slli_epi32::<N>(self.0)) }
    }
    #[inline(always)]
    pub fn eq(self, b: I8) -> M8 {
        unsafe { M8(_mm256_castsi256_ps(_mm256_cmpeq_epi32(self.0, b.0))) }
    }
    #[inline(always)]
    pub fn gt(self, b: I8) -> M8 {
        unsafe { M8(_mm256_castsi256_ps(_mm256_cmpgt_epi32(self.0, b.0))) }
    }
    #[inline(always)]
    pub fn lt(self, b: I8) -> M8 {
        b.gt(self)
    }
    /// Lanes not equal to `v`.
    #[inline(always)]
    pub fn ne_i(self, v: i32) -> M8 {
        !self.eq(I8::splat(v))
    }
    #[inline(always)]
    pub fn select(m: M8, t: I8, f: I8) -> I8 {
        unsafe { I8(_mm256_castps_si256(_mm256_blendv_ps(_mm256_castsi256_ps(f.0), _mm256_castsi256_ps(t.0), m.0))) }
    }
}

macro_rules! i8_ops {
    ($tr:ident, $f:ident, $intr:ident) => {
        impl $tr for I8 {
            type Output = I8;
            #[inline(always)]
            fn $f(self, b: I8) -> I8 {
                I8(unsafe { $intr(self.0, b.0) })
            }
        }
        impl $tr<i32> for I8 {
            type Output = I8;
            #[inline(always)]
            fn $f(self, b: i32) -> I8 {
                unsafe { I8(unsafe { $intr(self.0, _mm256_set1_epi32(b)) }) }
            }
        }
    };
}
i8_ops!(Add, add, _mm256_add_epi32);
i8_ops!(Sub, sub, _mm256_sub_epi32);
i8_ops!(BitAnd, bitand, _mm256_and_si256);
i8_ops!(BitOr, bitor, _mm256_or_si256);
i8_ops!(BitXor, bitxor, _mm256_xor_si256);

// ---- V8 -------------------------------------------------------------------

/// Eight 3-vectors (one per lane).
#[derive(Clone, Copy, Debug)]
pub struct V8 {
    pub x: F8,
    pub y: F8,
    pub z: F8,
}

impl V8 {
    #[inline(always)]
    pub fn new(x: F8, y: F8, z: F8) -> V8 {
        V8 { x, y, z }
    }
    #[inline(always)]
    pub fn splat(v: Vec3) -> V8 {
        V8 { x: F8::splat(v.x), y: F8::splat(v.y), z: F8::splat(v.z) }
    }
    #[inline(always)]
    pub fn zero() -> V8 {
        V8 { x: F8::zero(), y: F8::zero(), z: F8::zero() }
    }
    /// Lane i as a `Vec3`.
    #[inline(always)]
    pub fn lane(&self, i: usize) -> Vec3 {
        Vec3::new(self.x.lane(i), self.y.lane(i), self.z.lane(i))
    }
    #[inline(always)]
    pub fn dot(self, b: V8) -> F8 {
        self.z.mul_add(b.z, self.y.mul_add(b.y, self.x * b.x))
    }
    #[inline(always)]
    pub fn dot3(self, b: Vec3) -> F8 {
        self.z.mul_add(F8::splat(b.z), self.y.mul_add(F8::splat(b.y), self.x * b.x))
    }
    #[inline(always)]
    pub fn cross(self, b: V8) -> V8 {
        V8 { x: self.y * b.z - self.z * b.y, y: self.z * b.x - self.x * b.z, z: self.x * b.y - self.y * b.x }
    }
    #[inline(always)]
    pub fn length_sq(self) -> F8 {
        self.dot(self)
    }
    #[inline(always)]
    pub fn length(self) -> F8 {
        self.length_sq().sqrt()
    }
    /// Unit length (rsqrt plus one Newton step).
    #[inline(always)]
    pub fn normalize(self) -> V8 {
        self * self.length_sq().rsqrt()
    }
    /// self * s + b, per component
    #[inline(always)]
    pub fn mul_add(self, s: F8, b: V8) -> V8 {
        V8 { x: self.x.mul_add(s, b.x), y: self.y.mul_add(s, b.y), z: self.z.mul_add(s, b.z) }
    }
    /// o + d * t: the points along rays.
    #[inline(always)]
    pub fn at(o: V8, d: V8, t: F8) -> V8 {
        d.mul_add(t, o)
    }
    #[inline(always)]
    pub fn select(m: M8, t: V8, f: V8) -> V8 {
        V8 { x: F8::select(m, t.x, f.x), y: F8::select(m, t.y, f.y), z: F8::select(m, t.z, f.z) }
    }
    #[inline(always)]
    pub fn lerp(a: V8, b: V8, t: F8) -> V8 {
        V8 { x: F8::lerp(a.x, b.x, t), y: F8::lerp(a.y, b.y, t), z: F8::lerp(a.z, b.z, t) }
    }
    /// Component-wise product.
    #[inline(always)]
    pub fn hadamard(self, b: V8) -> V8 {
        V8 { x: self.x * b.x, y: self.y * b.y, z: self.z * b.z }
    }
    #[inline(always)]
    pub fn abs(self) -> V8 {
        V8 { x: self.x.abs(), y: self.y.abs(), z: self.z.abs() }
    }
    /// Per-component max with a scalar.
    #[inline(always)]
    pub fn max_s(self, s: F8) -> V8 {
        V8 { x: self.x.max(s), y: self.y.max(s), z: self.z.max(s) }
    }
    #[inline(always)]
    pub fn max_comp(self) -> F8 {
        self.x.max(self.y).max(self.z)
    }
    #[inline(always)]
    pub fn one() -> V8 {
        V8::splat(Vec3::ONE)
    }
    /// Reflection of self about the unit normal n.
    #[inline(always)]
    pub fn reflect(self, n: V8) -> V8 {
        let k = self.dot(n) * -2.0;
        n.mul_add(k, self)
    }
}

impl Add for V8 {
    type Output = V8;
    #[inline(always)]
    fn add(self, b: V8) -> V8 {
        V8 { x: self.x + b.x, y: self.y + b.y, z: self.z + b.z }
    }
}
impl Sub for V8 {
    type Output = V8;
    #[inline(always)]
    fn sub(self, b: V8) -> V8 {
        V8 { x: self.x - b.x, y: self.y - b.y, z: self.z - b.z }
    }
}
impl Mul<F8> for V8 {
    type Output = V8;
    #[inline(always)]
    fn mul(self, s: F8) -> V8 {
        V8 { x: self.x * s, y: self.y * s, z: self.z * s }
    }
}
impl Mul<f32> for V8 {
    type Output = V8;
    #[inline(always)]
    fn mul(self, s: f32) -> V8 {
        V8 { x: self.x * s, y: self.y * s, z: self.z * s }
    }
}
impl Add<Vec3> for V8 {
    type Output = V8;
    #[inline(always)]
    fn add(self, b: Vec3) -> V8 {
        V8 { x: self.x + b.x, y: self.y + b.y, z: self.z + b.z }
    }
}
impl Sub<Vec3> for V8 {
    type Output = V8;
    #[inline(always)]
    fn sub(self, b: Vec3) -> V8 {
        V8 { x: self.x - b.x, y: self.y - b.y, z: self.z - b.z }
    }
}
impl Neg for V8 {
    type Output = V8;
    #[inline(always)]
    fn neg(self) -> V8 {
        V8 { x: -self.x, y: -self.y, z: -self.z }
    }
}
impl AddAssign for V8 {
    #[inline(always)]
    fn add_assign(&mut self, b: V8) {
        *self = *self + b;
    }
}
