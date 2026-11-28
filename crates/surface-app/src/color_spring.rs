//! Retained color interpolation for the Material Standard FastEffects spring.

use crate::motion::ScalarSpring;
use surface::Color;

// StandardMotionTokens.kt at AndroidX 26065e0db6b4c61d1779c89ad388ea75ce98235a.
const STIFFNESS: f32 = 3800.;
const DAMPING: f32 = 1.;

// ScalarSpring settles at 0.01 units. Give each normalized channel the
// scale of an 8-bit channel so this threshold does not snap small visible
// color changes early. Store velocity in continuous channel units.
const CHANNEL_SCALE: f64 = 255.;

pub(crate) struct ColorSpring {
    target: Color,
    components: [f64; 4],
    springs: [Option<ScalarSpring>; 4],
    last_time_ms: Option<f64>,
}

impl ColorSpring {
    pub(crate) fn new(target: Color) -> Self {
        Self {
            target,
            components: to_oklab(target),
            springs: [None; 4],
            last_time_ms: None,
        }
    }

    /// Return the presented sRGB color and whether another frame is needed.
    pub(crate) fn sample(
        &mut self,
        target: Color,
        now_ms: Option<f64>,
        snap: bool,
    ) -> (Color, bool) {
        let Some(now_ms) = now_ms.filter(|time| {
            time.is_finite() && !snap && self.last_time_ms.is_none_or(|last| *time >= last)
        }) else {
            if target != self.target {
                *self = Self::new(target);
            } else {
                self.springs = [None; 4];
                self.last_time_ms = None;
            }
            return (target, false);
        };
        self.last_time_ms = Some(now_ms);

        if target != self.target {
            let components = to_oklab(target);
            self.springs = std::array::from_fn(|index| {
                let [position, velocity] = self
                    .channel_pose(index, now_ms)
                    .unwrap_or([self.components[index], 0.]);
                let offset = position - components[index];
                (offset != 0. || velocity != 0.).then(|| {
                    ScalarSpring::new(
                        STIFFNESS,
                        DAMPING,
                        [1. + offset * CHANNEL_SCALE, velocity * CHANNEL_SCALE],
                        now_ms,
                    )
                })
            });
            self.target = target;
            self.components = components;
        }

        let mut components = self.components;
        let mut running = false;
        for (index, component) in components.iter_mut().enumerate() {
            if let Some([position, _]) = self.channel_pose(index, now_ms) {
                *component = position;
                running = true;
            } else {
                self.springs[index] = None;
            }
        }
        if running {
            (from_oklab(components), true)
        } else {
            (self.target, false)
        }
    }

    fn channel_pose(&self, index: usize, now_ms: f64) -> Option<[f64; 2]> {
        let [position, velocity] = self.springs[index]?.sample(now_ms)?;
        Some([
            self.components[index] + (position - 1.) / CHANNEL_SCALE,
            velocity / CHANNEL_SCALE,
        ])
    }
}

// D65 linear-sRGB/Oklab matrices from Björn Ottosson's public-domain
// reference at https://bottosson.github.io/posts/oklab/#implementation.
// The sRGB transfer uses the IEC 61966-2-1 cutoffs. AndroidX's color
// animation also keeps alpha separate from the three Oklab components.
fn to_oklab(Color(red, green, blue, alpha): Color) -> [f64; 4] {
    let [red, green, blue] = [red, green, blue].map(|channel| {
        let srgb = f64::from(channel) / 255.;
        if srgb <= 0.04045 {
            srgb / 12.92
        } else {
            ((srgb + 0.055) / 1.055).powf(2.4)
        }
    });
    let l = (0.412_221_470_8 * red + 0.536_332_536_3 * green + 0.051_445_992_9 * blue).cbrt();
    let m = (0.211_903_498_2 * red + 0.680_699_545_1 * green + 0.107_396_956_6 * blue).cbrt();
    let s = (0.088_302_461_9 * red + 0.281_718_837_6 * green + 0.629_978_700_5 * blue).cbrt();
    [
        0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766 * s,
        f64::from(alpha) / 255.,
    ]
}

fn from_oklab([lightness, a, b, alpha]: [f64; 4]) -> Color {
    let l = (lightness + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let m = (lightness - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let s = (lightness - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3);
    let [red, green, blue] = [
        4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
        -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
        -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701 * s,
    ]
    .map(|linear| {
        let srgb = if linear <= 0.0031308 {
            12.92 * linear
        } else {
            1.055 * linear.powf(1. / 2.4) - 0.055
        };
        quantize(srgb)
    });
    Color(red, green, blue, quantize(alpha))
}

fn quantize(channel: f64) -> u8 {
    (channel.clamp(0., 1.) * 255.).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_uses_the_fast_effects_reference_curve_and_exact_endpoints() {
        let idle = Color(73, 69, 78, 64);
        let focus = Color(101, 85, 143, 224);
        let mut motion = ColorSpring::new(idle);
        assert_eq!(motion.sample(idle, Some(0.), false), (idle, false));
        assert_eq!(motion.sample(focus, Some(100.), false), (idle, true));
        // Independent CSS Color 4 XYZ/Oklab matrices and the analytic
        // 1 - (1 + sqrt(3800) * t) * exp(-sqrt(3800) * t) response.
        for (time, color) in [
            (114., Color(79, 73, 92, 98)),
            (125., Color(85, 77, 107, 137)),
            (144., Color(94, 81, 127, 185)),
        ] {
            assert_eq!(motion.sample(focus, Some(time), false), (color, true));
        }
        assert_eq!(motion.sample(focus, Some(600.), false), (focus, false));
        assert_eq!(motion.sample(focus, Some(616.), false), (focus, false));
    }

    #[test]
    fn reversing_and_retargeting_keep_the_continuous_pose_and_velocity() {
        let idle = Color(73, 69, 78, 255);
        let focus = Color(101, 85, 143, 255);
        let error = Color(186, 26, 26, 255);
        let mut motion = ColorSpring::new(idle);
        motion.sample(focus, Some(100.), false);
        let (mid, _) = motion.sample(focus, Some(125.), false);
        let before = motion.channel_pose(0, 125.).unwrap();
        assert!(before[1] > 0.);
        assert_eq!(motion.sample(idle, Some(125.), false), (mid, true));
        let after = motion.channel_pose(0, 125.).unwrap();
        for (before, after) in before.into_iter().zip(after) {
            assert!((before - after).abs() < 1e-12);
        }
        let (turning, _) = motion.sample(idle, Some(131.), false);
        assert_eq!(motion.sample(error, Some(131.), false), (turning, true));
        assert_eq!(motion.sample(error, Some(640.), false), (error, false));
    }

    #[test]
    fn alpha_has_its_own_track_and_invalid_clocks_or_snap_reset_it() {
        let faint = Color(16, 32, 224, 0);
        let opaque = Color(16, 32, 224, 255);
        let mut motion = ColorSpring::new(faint);
        assert_eq!(motion.sample(opaque, Some(10.), false), (faint, true));
        assert_eq!(
            motion.sample(opaque, Some(35.), false),
            (Color(16, 32, 224, 116), true)
        );
        assert_eq!(motion.sample(opaque, None, false), (opaque, false));
        assert_eq!(motion.sample(faint, Some(40.), true), (faint, false));
        assert_eq!(
            motion.sample(opaque, Some(f64::NAN), false),
            (opaque, false)
        );
        assert_eq!(motion.sample(faint, Some(100.), false), (opaque, true));
        assert_eq!(motion.sample(faint, Some(99.), false), (faint, false));
    }
}
