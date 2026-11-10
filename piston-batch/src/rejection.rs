/*!
# FSM and ASM rejection transfer functions

Discrete time models (1kHz) of the M2 feedback loops applied to the wind loads
M1 & M2 rigid body motions (RBMs), ported from
[wind2optics](https://github.com/GMTO-Integrated-Modeling/wind2optics)
(`m2_control_system.rs`, `control.rs` and `WindLoadedGmt`).

 - [tip-tilt loop](Rejection::tiptilt_loop): the segment tip-tilt (LOM) is fed to the
   FSM or ASM controller and corrected with M2 segment Rx & Ry on the next sample,
 - [piston loop](Rejection::piston_loop) (ASM only): the segment piston is corrected
   with the edge sensors and the optical piston sensor loops.
*/

use anyhow::anyhow;
use gmt_lom::{OpticalSensitivities, OpticalSensitivity};
use nalgebra as na;

const N_SEGMENT: usize = 7;

/// M2 type
#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
pub enum M2 {
    FSM,
    ASM,
}
impl M2 {
    /// Suffix of the time series and statistics columns
    pub fn suffix(&self) -> &'static str {
        match self {
            M2::FSM => "_FSM",
            M2::ASM => "_ASM",
        }
    }
}

trait Control {
    fn step(&mut self, value: &[f64]) -> Option<Vec<f64>>;
    fn last(&self) -> Option<Vec<f64>> {
        None
    }
}

/// Averages `n_sample` consecutive samples
struct Average {
    n_sample: usize,
    average: Vec<f64>,
    counter: usize,
}
impl Average {
    fn new(n_sample: usize, n_data: usize) -> Self {
        Self {
            n_sample,
            average: vec![0f64; n_data],
            counter: 0,
        }
    }
}
impl Control for Average {
    fn step(&mut self, value: &[f64]) -> Option<Vec<f64>> {
        self.average
            .iter_mut()
            .zip(value)
            .for_each(|(a, v)| *a += *v);
        self.counter += 1;
        if self.counter == self.n_sample {
            let mean = self
                .average
                .iter()
                .map(|x| x / self.n_sample as f64)
                .collect();
            self.average.iter_mut().for_each(|a| *a = 0f64);
            self.counter = 0;
            Some(mean)
        } else {
            None
        }
    }
}

/// `y[k] = y[k-1] + g u[k]`
struct Integrate {
    gain: f64,
    mem: Vec<f64>,
}
impl Integrate {
    fn new(gain: f64, n_data: usize) -> Self {
        Self {
            gain,
            mem: vec![0f64; n_data],
        }
    }
}
impl Control for Integrate {
    fn step(&mut self, value: &[f64]) -> Option<Vec<f64>> {
        let gain = self.gain;
        self.mem
            .iter_mut()
            .zip(value)
            .for_each(|(a, v)| *a += *v * gain);
        self.last()
    }
    fn last(&self) -> Option<Vec<f64>> {
        Some(self.mem.clone())
    }
}

/// `y[k] = g u[k]`
struct Proportional {
    gain: f64,
    mem: Vec<f64>,
}
impl Proportional {
    fn new(gain: f64, n_data: usize) -> Self {
        Self {
            gain,
            mem: vec![0f64; n_data],
        }
    }
}
impl Control for Proportional {
    fn step(&mut self, value: &[f64]) -> Option<Vec<f64>> {
        let gain = self.gain;
        self.mem
            .iter_mut()
            .zip(value)
            .for_each(|(a, v)| *a = *v * gain);
        self.last()
    }
    fn last(&self) -> Option<Vec<f64>> {
        Some(self.mem.clone())
    }
}

/// `x[k] = x[k-1] + g u[k]`, `y[k] = u[k] + x[k]`
struct ProportionalIntegral {
    gain: f64,
    state: Vec<f64>,
    mem: Vec<f64>,
}
impl ProportionalIntegral {
    fn new(gain: f64, n_data: usize) -> Self {
        Self {
            gain,
            state: vec![0f64; n_data],
            mem: vec![0f64; n_data],
        }
    }
}
impl Control for ProportionalIntegral {
    fn step(&mut self, value: &[f64]) -> Option<Vec<f64>> {
        let gain = self.gain;
        self.state
            .iter_mut()
            .zip(value)
            .for_each(|(a, v)| *a += *v * gain);
        self.mem
            .iter_mut()
            .zip(&self.state)
            .zip(value)
            .for_each(|((a, s), v)| *a = *v + *s);
        self.last()
    }
    fn last(&self) -> Option<Vec<f64>> {
        Some(self.mem.clone())
    }
}

/// `n_sample` samples delay
struct Delay {
    mem: Vec<Vec<f64>>,
}
impl Delay {
    fn new(n_sample: usize, n_data: usize) -> Self {
        Self {
            mem: vec![vec![0f64; n_data]; n_sample + 1],
        }
    }
}
impl Control for Delay {
    fn step(&mut self, value: &[f64]) -> Option<Vec<f64>> {
        self.mem[0].copy_from_slice(value);
        self.mem.rotate_right(1);
        Some(self.mem[0].clone())
    }
}

/// Second order discrete state space model, one per data channel
struct StateSpace2x2 {
    a: [f64; 4],
    b: [f64; 2],
    c: [f64; 2],
    d: f64,
    x: Vec<(f64, f64)>,
}
impl StateSpace2x2 {
    fn new((a, b, c, d): ([f64; 4], [f64; 2], [f64; 2], f64), n_data: usize) -> Self {
        Self {
            a,
            b,
            c,
            d,
            x: vec![(0f64, 0f64); n_data],
        }
    }
}
impl Control for StateSpace2x2 {
    fn step(&mut self, u: &[f64]) -> Option<Vec<f64>> {
        let Self { a, b, c, d, x } = self;
        Some(
            u.iter()
                .zip(x.iter_mut())
                .map(|(u, x)| {
                    let y = c[0] * x.0 + c[1] * x.1 + *d * u;
                    *x = (
                        a[0] * x.0 + a[1] * x.1 + b[0] * u,
                        a[2] * x.0 + a[3] * x.1 + b[1] * u,
                    );
                    y
                })
                .collect(),
        )
    }
}

enum Controller {
    Proportional(Proportional),
    Integral(Integrate),
    /// integral followed by a proportional-integral
    SecondOrder(Integrate, ProportionalIntegral),
}
impl Control for Controller {
    fn step(&mut self, value: &[f64]) -> Option<Vec<f64>> {
        match self {
            Self::Proportional(c) => c.step(value),
            Self::Integral(c) => c.step(value),
            Self::SecondOrder(i, pi) => i.step(value).and_then(|value| pi.step(&value)),
        }
    }
    fn last(&self) -> Option<Vec<f64>> {
        match self {
            Self::Proportional(c) => c.last(),
            Self::Integral(c) => c.last(),
            Self::SecondOrder(_, pi) => pi.last(),
        }
    }
}

/// M2 control loop: sensor averaging, controller, latency and actuator dynamics
struct M2Loop {
    sensor: Average,
    controller: Controller,
    delay: Delay,
    dynamics: StateSpace2x2,
}
/// ASM actuator dynamics used by the piston loops (800Hz, ζ=0.75)
const ASM_PISTON_DYNAMICS: ([f64; 4], [f64; 2], [f64; 2], f64) = (
    [-0.95910647, -0.31990701, 1., 0.],
    [1., 0.],
    [0.5930526, 0.38748527],
    0.56975337,
);
impl M2Loop {
    /// FSM tip-tilt loop: 5 samples average, 6 samples latency, 25Hz dynamics (ζ=1/√2)
    fn fsm(n: usize) -> Self {
        Self {
            sensor: Average::new(5, n),
            controller: Controller::SecondOrder(
                Integrate::new(0.1376, n),
                ProportionalIntegral::new(0.0719, n),
            ),
            delay: Delay::new(6, n),
            dynamics: StateSpace2x2::new(
                (
                    [1.77908235, -0.80116713, 1., 0.],
                    [1., 0.],
                    [0.02086505, 0.00109779],
                    0.00552119,
                ),
                n,
            ),
        }
    }
    /// ASM tip-tilt loop: 2 samples average, 1 sample latency, 800Hz dynamics (ζ=0.7)
    fn asm(n: usize) -> Self {
        Self {
            sensor: Average::new(2, n),
            controller: Controller::SecondOrder(
                Integrate::new(0.35, n),
                ProportionalIntegral::new(0.2738, n),
            ),
            delay: Delay::new(1, n),
            dynamics: StateSpace2x2::new(
                (
                    [-0.98135353, -0.35052305, 1., 0.],
                    [1., 0.],
                    [0.59383946, 0.37862502],
                    0.58296915,
                ),
                n,
            ),
        }
    }
    /// Optical piston sensor loop: 100 samples average, 50 samples latency, integral gain 0.5
    fn piston_optical_sensor(n: usize) -> Self {
        Self {
            sensor: Average::new(100, n),
            controller: Controller::Integral(Integrate::new(0.5, n)),
            delay: Delay::new(50, n),
            dynamics: StateSpace2x2::new(ASM_PISTON_DYNAMICS, n),
        }
    }
    /// Edge sensors piston loop: 2 samples average, no latency, proportional gain 0.8
    fn piston_edge_sensor(n: usize) -> Self {
        Self {
            sensor: Average::new(2, n),
            controller: Controller::Proportional(Proportional::new(0.8, n)),
            delay: Delay::new(0, n),
            dynamics: StateSpace2x2::new(ASM_PISTON_DYNAMICS, n),
        }
    }
    fn step(&mut self, input: &[f64]) -> Vec<f64> {
        let value = match self.sensor.step(input) {
            Some(value) => self.controller.step(&value),
            None => self.controller.last(),
        }
        .unwrap();
        let value = self.delay.step(&value).unwrap();
        self.dynamics.step(&value).unwrap()
    }
}

/// FSM and ASM rejection of the wind loads RBMs
pub struct Rejection {
    /// segment tip-tilt sensitivity `[14x84]`
    tiptilt: na::DMatrix<f64>,
    /// inverse of the M2 Rx & Ry to segment tip-tilt sensitivity `[14x14]`
    inv_m2_rxy: na::DMatrix<f64>,
    /// segment piston sensitivity `[7x84]`
    piston: na::DMatrix<f64>,
}
impl Rejection {
    pub fn new(sensitivities: &OpticalSensitivities) -> anyhow::Result<Self> {
        let tiptilt = &sensitivities[OpticalSensitivity::SegmentTipTilt(vec![])];
        let inv_m2_rxy = tiptilt
            .m2_rxy()?
            .qr()
            .try_inverse()
            .ok_or_else(|| anyhow!("M2 Rxy segment tip-tilt sensitivity is singular"))?;
        Ok(Self {
            tiptilt: tiptilt.into(),
            inv_m2_rxy,
            piston: (&sensitivities[OpticalSensitivity::SegmentPiston(vec![])]).into(),
        })
    }
    /// Segment piston `[7xn]` of the RBMs `[84xn]`
    pub fn segment_piston(&self, rbm: &na::DMatrix<f64>) -> na::DMatrix<f64> {
        &self.piston * rbm
    }
    /// Corrects in place the RBMs `[84xn]` with the M2 tip-tilt feedback loop
    ///
    /// The M2 segment Rx & Ry corrections derived from sample k are applied to sample k+1.
    pub fn tiptilt_loop(&self, rbm: &mut na::DMatrix<f64>, m2: M2) {
        let mut m2_loop = match m2 {
            M2::FSM => M2Loop::fsm(2 * N_SEGMENT),
            M2::ASM => M2Loop::asm(2 * N_SEGMENT),
        };
        // M2 segment Rx & Ry rows
        let idx: Vec<_> = (0..N_SEGMENT)
            .flat_map(|i| [45 + 6 * i, 46 + 6 * i])
            .collect();
        let mut m2_rxy = na::DVector::<f64>::zeros(2 * N_SEGMENT);
        for mut rbm in rbm.column_iter_mut() {
            idx.iter()
                .zip(m2_rxy.iter())
                .for_each(|(&i, r)| rbm[i] -= r);
            let tiptilt = &self.tiptilt * &rbm;
            let command = na::DVector::from_vec(m2_loop.step(tiptilt.as_slice()));
            m2_rxy = &self.inv_m2_rxy * command;
        }
    }
    /// Returns the residual segment piston `[7xn]` of the ASM piston feedback loop
    ///
    /// The piston command is the sum of the edge sensors loop output, driven by the
    /// segment piston, and of the optical piston sensor loop output, driven by the
    /// residual segment piston.
    pub fn piston_loop(&self, rbm: &na::DMatrix<f64>) -> na::DMatrix<f64> {
        let mut edge_sensor = M2Loop::piston_edge_sensor(N_SEGMENT);
        let mut optical_sensor = M2Loop::piston_optical_sensor(N_SEGMENT);
        let mut residual = self.segment_piston(rbm);
        let mut command = vec![0f64; N_SEGMENT];
        for mut piston in residual.column_iter_mut() {
            let es = edge_sensor.step(piston.as_slice());
            piston.iter_mut().zip(&command).for_each(|(p, c)| *p -= c);
            let os = optical_sensor.step(piston.as_slice());
            command
                .iter_mut()
                .zip(es.into_iter().zip(os))
                .for_each(|(c, (es, os))| *c = es + os);
        }
        residual
    }
    /// Replaces in place the M2 segment Tz of the RBMs `[84xn]` such as the segment
    /// piston of the RBMs matches the given segment `piston` `[7xn]`
    pub fn fit_m2_piston(
        &self,
        rbm: &mut na::DMatrix<f64>,
        piston: &na::DMatrix<f64>,
    ) -> anyhow::Result<()> {
        let m1_piston = self.piston.columns(0, 42) * rbm.rows(0, 42);
        let m2_rbm = self
            .piston
            .columns(42, 42)
            .into_owned()
            .pseudo_inverse(0.)
            .map_err(|e| anyhow!(e))?
            * (piston - m1_piston);
        for i in 0..N_SEGMENT {
            let row = 6 * i + 2;
            rbm.row_mut(42 + row).copy_from(&m2_rbm.row(row));
        }
        Ok(())
    }
}
