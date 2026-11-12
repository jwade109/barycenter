use crate::*;
use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub struct ThrustAxisControl {
    pub use_rcs: bool,
    pub throttle: f32,
}

impl ThrustAxisControl {
    pub const NULLOPT: ThrustAxisControl = ThrustAxisControl {
        use_rcs: false,
        throttle: 0.0,
    };
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub struct VehicleControl {
    pub plus_x: ThrustAxisControl,
    pub plus_y: ThrustAxisControl,
    pub neg_x: ThrustAxisControl,
    pub neg_y: ThrustAxisControl,
    pub attitude: f64,
}

impl VehicleControl {
    pub const NULLOPT: Self = Self {
        plus_x: ThrustAxisControl::NULLOPT,
        plus_y: ThrustAxisControl::NULLOPT,
        neg_x: ThrustAxisControl::NULLOPT,
        neg_y: ThrustAxisControl::NULLOPT,
        attitude: 0.0,
    };

    pub const FORWARD_MAIN: Self = Self {
        plus_x: ThrustAxisControl {
            use_rcs: false,
            throttle: 1.0,
        },
        plus_y: ThrustAxisControl::NULLOPT,
        neg_x: ThrustAxisControl::NULLOPT,
        neg_y: ThrustAxisControl::NULLOPT,
        attitude: 0.0,
    };

    pub fn rcs(forward: bool, left: bool, right: bool, backward: bool) -> Self {
        Self {
            plus_x: ThrustAxisControl {
                use_rcs: true,
                throttle: forward as u8 as f32,
            },
            plus_y: ThrustAxisControl {
                use_rcs: true,
                throttle: left as u8 as f32,
            },
            neg_x: ThrustAxisControl {
                use_rcs: true,
                throttle: backward as u8 as f32,
            },
            neg_y: ThrustAxisControl {
                use_rcs: true,
                throttle: right as u8 as f32,
            },
            attitude: 0.0,
        }
    }

    pub fn is_nullopt(&self) -> bool {
        self.plus_x.throttle == 0.0
            && self.plus_y.throttle == 0.0
            && self.neg_x.throttle == 0.0
            && self.neg_y.throttle == 0.0
            && self.attitude == 0.0
    }
}

pub fn zero_gravity_velocity_control_law(
    desired_vel: DVec2,
    idle_angle: f64,
    body: &RigidBody,
    attitude_pd: &PDCtrl,
) -> (VehicleControl, VehicleControlStatus) {
    let vel_error = body.pv.vel - desired_vel;
    let mut ctrl = VehicleControl::NULLOPT;

    let target_angle = idle_angle;

    // let target_angle = if vel_error.length() > 0.5 {
    //     // ctrl.plus_x.throttle = (0.2 + vel_error.length() / 10.0).clamp(0.0, 1.0) as f32;
    //     let target_angle = (-vel_error).to_angle();
    //     let attitude_error = wrap_pi_npi_f64(target_angle - body.angle);
    //     if attitude_error.to_degrees().abs() < 5.0 {
    //         ctrl.plus_x.throttle = 1.0;
    //     }

    //     target_angle
    // } else {
    //     idle_angle
    // };

    let verror = SetpointError {
        target: desired_vel,
        actual: body.pv.vel,
        error: vel_error,
    };

    // if vel_error.length() > 0.05 && vel_error.length() < 1.0 {
    let body_frame_error = rotate_f64(vel_error, -body.angle);
    if body_frame_error.x < -0.0001 {
        ctrl.plus_x.throttle = body_frame_error.x.abs() as f32;
        ctrl.plus_x.use_rcs = true;
    }
    if body_frame_error.x > 0.0001 {
        ctrl.neg_x.throttle = body_frame_error.x.abs() as f32;
        ctrl.neg_x.use_rcs = true;
    }
    if body_frame_error.y > 0.0001 {
        ctrl.plus_y.throttle = body_frame_error.y.abs() as f32;
        ctrl.plus_y.use_rcs = true;
    }
    if body_frame_error.y < -0.0001 {
        ctrl.neg_y.throttle = body_frame_error.y.abs() as f32;
        ctrl.neg_y.use_rcs = true;
    }
    // }

    let (attitude, herror) = compute_attitude_control(body, target_angle, attitude_pd);

    ctrl.attitude = attitude;

    (
        ctrl,
        VehicleControlStatus::VelocityHold {
            vel: verror,
            hdg: herror,
        },
    )
}

pub fn zero_gravity_control_law(
    target: PV,
    target_angle: f64,
    body: &RigidBody,
    attitude_pd: &PDCtrl,
) -> (VehicleControl, VehicleControlStatus) {
    let error = target - body.pv;
    let distance = error.pos.length();
    let error_hat = error.pos.normalize_or_zero();
    let (desired_magnitude, desired_angle) = if distance > 7000.0 {
        (250.0, body.angle)
    } else if distance > 2000.0 {
        (50.0, body.angle)
    } else if distance > 1000.0 {
        (20.0, body.angle)
    } else if distance > 40.0 {
        (5.0, body.angle)
    } else {
        (distance / 30.0, target_angle)
    };

    let pos = SetpointError {
        target: target.pos,
        actual: body.pv.pos,
        error: error.pos,
    };

    let desired_vel = target.vel + error_hat * desired_magnitude;
    let (ctrl, status) =
        zero_gravity_velocity_control_law(desired_vel, desired_angle, body, attitude_pd);

    let status = match status {
        VehicleControlStatus::VelocityHold { vel, hdg } => {
            VehicleControlStatus::PositionHold { pos, vel, hdg }
        }
        _ => unimplemented!(),
    };

    (ctrl, status)
}

fn compute_attitude_control(
    body: &RigidBody,
    target_angle: f64,
    pd: &PDCtrl,
) -> (f64, SetpointError<f64>) {
    let attitude_error = wrap_pi_npi_f64(target_angle - body.angle);

    let herror = SetpointError {
        target: target_angle,
        actual: body.angle,
        error: attitude_error,
    };

    // if wrap_pi_npi_f64(target_angle - body.angle).abs() < 0.02 {
    //     return (0.0, herror);
    // }

    let x = pd.apply(attitude_error, body.angular_velocity);
    // let x = if x.abs() > 1.0 { x } else { 0.0 };

    (x, herror)
}

pub fn attitude_control_law(
    target_angle: f64,
    pid: &PDCtrl,
    body: &RigidBody,
) -> (VehicleControl, VehicleControlStatus) {
    let mut cmd = VehicleControl::NULLOPT;
    (cmd.attitude, _) = compute_attitude_control(body, target_angle, pid);
    let attitude_error = wrap_pi_npi_f64(target_angle - body.angle);

    let error = SetpointError {
        target: target_angle,
        actual: body.angle,
        error: attitude_error,
    };

    (cmd, VehicleControlStatus::AttitudeHold { error })
}

fn hover_control_law(
    _target: DVec2,
    _gravity: DVec2,
    // _vehicle: &Blueprint,
    _body: &RigidBody,
) -> (VehicleControl, VehicleControlStatus) {
    unimplemented!()

    // let upright_angle = (-gravity).to_angle();

    // let target = if target.distance(body.pv.pos) > 250.0 {
    //     let d = target - body.pv.pos;
    //     d.normalize_or_zero() * 250.0 + body.pv.pos
    // } else {
    //     target
    // };

    // let horizontal_control =
    //     PLACEHOLDER_PD.apply(target.x - body.pv.pos.x as f64, body.pv.vel.x as f64);

    // // attitude controller
    // let target_angle = upright_angle - horizontal_control.clamp(-PI_64 / 6.0, PI_64 / 6.0);
    // let attitude_error = (body.angle - target_angle).abs();
    // let (attitude, _) = compute_attitude_control(body, target_angle, &PLACEHOLDER_PD);

    // let thrust = vehicle.max_forward_thrust();
    // let accel = thrust / MASS_PLACEHOLDER.to_kg_f64();
    // let pct = gravity.length() / accel;

    // // vertical controller
    // let error = PLACEHOLDER_PD.apply(target.y - body.pv.pos.y as f64, body.pv.vel.y as f64);

    // let throttle = pct + error;

    // let mut ctrl = VehicleControl::NULLOPT;

    // let status = if attitude_error < 0.7 {
    //     ctrl.plus_x.throttle = throttle as f32;
    //     VehicleControlStatus::TurningToHover
    // } else {
    //     VehicleControlStatus::Hovering
    // };

    // ctrl.attitude = attitude;

    // (ctrl, status)
}

pub fn position_hold_control_law(
    target: PV,
    target_angle: f64,
    body: &RigidBody,
    gravity: DVec2,
    attitude_pd: &PDCtrl,
) -> (VehicleControl, VehicleControlStatus) {
    if gravity.length() > 0.0 {
        hover_control_law(target.pos, gravity, body)
    } else {
        zero_gravity_control_law(target, target_angle, body, attitude_pd)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize, Serialize)]
pub struct SetpointError<T, E = T> {
    target: T,
    actual: T,
    error: E,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize, Serialize)]
pub enum VehicleControlStatus {
    Done,
    WaitingForInput,
    UnderExternalControl,
    RaisingOrbit,
    StopFalling,
    RaisingPeriapsis(i32),
    ExecutingLaunchProgram,
    CoastingToApoapsis(i32),
    #[default]
    Idling,
    Whatever,
    InProgress,
    TurningToHover,
    Hovering,
    NoVelocityVector,
    ComingAbout,
    HoldingAttitude,
    HoldingPosition,
    NoParentBody,
    FinalApproach,
    Arrived,
    CorrectingBadVelocity,
    DriftingTowardsTarget,
    BrakingBurn,
    FlipAndBurn,
    AttitudeHold {
        error: SetpointError<f64>,
    },
    VelocityHold {
        vel: SetpointError<DVec2>,
        hdg: SetpointError<f64>,
    },
    PositionHold {
        pos: SetpointError<DVec2>,
        vel: SetpointError<DVec2>,
        hdg: SetpointError<f64>,
    },
}

impl VehicleControlStatus {
    pub fn is_done(&self) -> bool {
        match self {
            VehicleControlStatus::Done => true,
            _ => false,
        }
    }

    pub fn is_awaiting_user_input(&self) -> bool {
        match self {
            VehicleControlStatus::WaitingForInput => true,
            _ => false,
        }
    }
}

fn to_int_percent(x: f64) -> i32 {
    (100.0 * x).round() as i32
}

pub fn enter_orbit_control_law(
    planet: &Body,
    body: &RigidBody,
    orbit: Option<&SparseOrbit>,
    target_altitude: f64,
    attitude: &PDCtrl,
) -> (VehicleControl, VehicleControlStatus) {
    let target_apoapsis = target_altitude + 10000.0;
    let target_periapsis = target_altitude;

    let altitude = body.pv.pos.length() - planet.radius;
    let vertical = body.pv.pos.to_angle();
    let vertical_velocity = body.pv.vel.dot(body.pv.pos.normalize_or_zero());
    let gravity = planet.gravity(body.pv.pos).length();

    let (apoapsis_altitude, periapsis_altitude, circular) = if let Some(orbit) = orbit {
        (
            orbit.apoapsis_r() - planet.radius,
            orbit.periapsis_r() - planet.radius,
            match orbit.class() {
                OrbitClass::Circular => true,
                OrbitClass::NearCircular => true,
                _ => false,
            },
        )
    } else {
        (
            kinematic_apoapis(altitude, vertical_velocity, gravity),
            0.0,
            false,
        )
    };

    let att_and_throttle = |target_angle: f64, throttle: f32| {
        let mut cmd = VehicleControl::NULLOPT;
        (cmd.attitude, _) = compute_attitude_control(body, target_angle, attitude);
        // let angle_error = wrap_pi_npi_f64(target_angle - body.angle);
        // if angle_error.abs() < 0.1 {
        cmd.plus_x.throttle = throttle;
        // }
        cmd
    };

    let near_ground = altitude < 20_000.0;
    let falling = vertical_velocity < 0.0;
    let periapsis_above_target = periapsis_altitude > target_periapsis;
    let apoapsis_above_target = apoapsis_altitude > target_apoapsis;
    let above_target = altitude > target_altitude;

    let launch_program_target_angle = {
        let start_altitude = 1000.0;
        let end_altitude = 12000.0;
        let s = ((altitude - start_altitude) / end_altitude).clamp(0.0, 1.0);
        let off_vertical = s * PI_64 / 2.0;
        vertical + off_vertical
    };

    let circularization_angle = {
        let off_horizontal_angle = (vertical_velocity / 100.0).clamp(-PI_64 / 5.0, PI_64 / 5.0);
        vertical + PI_64 / 2.0 + off_horizontal_angle
    };

    let (cmd, status) = if apoapsis_above_target && periapsis_above_target {
        (VehicleControl::NULLOPT, VehicleControlStatus::Done)
    } else if near_ground && falling {
        (
            att_and_throttle(vertical, 1.0),
            VehicleControlStatus::StopFalling,
        )
    } else if apoapsis_above_target || above_target {
        if !above_target {
            (
                VehicleControl::NULLOPT,
                VehicleControlStatus::CoastingToApoapsis(to_int_percent(
                    altitude / target_altitude,
                )),
            )
        } else if !periapsis_above_target || !circular {
            (
                att_and_throttle(circularization_angle, 0.2),
                VehicleControlStatus::RaisingPeriapsis(to_int_percent(
                    periapsis_altitude / target_periapsis,
                )),
            )
        } else {
            (VehicleControl::NULLOPT, VehicleControlStatus::InProgress)
        }
    } else {
        // TODO big placeholder energy
        let max_accel = 5.0;
        let target_accel = 16.0;
        let throttle = target_accel / max_accel;
        (
            att_and_throttle(launch_program_target_angle, throttle as f32),
            VehicleControlStatus::ExecutingLaunchProgram,
        )
    };

    (cmd, status)
}

pub fn burn_along_velocity_vector_control_law(
    body: &RigidBody,
    prograde: bool,
    attitude: &PDCtrl,
) -> (VehicleControl, VehicleControlStatus) {
    if body.pv.vel.length() < 0.2 {
        return (
            VehicleControl::NULLOPT,
            VehicleControlStatus::NoVelocityVector,
        );
    }

    let thrust_angle = if prograde { body.pv.vel } else { -body.pv.vel }.to_angle();
    let mut ctrl = VehicleControl::NULLOPT;
    let actual_angle = body.angle;
    (ctrl.attitude, _) = compute_attitude_control(body, thrust_angle, attitude);
    let angular_error = wrap_pi_npi_f64((thrust_angle - actual_angle).abs());
    let status = if angular_error.abs().to_degrees() < 3.0
        && body.angular_velocity.to_degrees().abs() < 3.0
    {
        ctrl.plus_x.throttle = 0.5;
        VehicleControlStatus::InProgress
    } else {
        VehicleControlStatus::ComingAbout
    };

    (ctrl, status)
}

#[derive(Debug, Clone, PartialEq)]
pub enum VehicleControlPolicy {
    Idle,
    External,
    PositionHold(Vec<(DVec2, f64)>),
    LaunchToOrbit(f64),
    BurnPrograde,
    BurnRetrograde,
    HoldAttitude(Option<f64>),
    BurnNorth,
    BurnSouth,
    BurnEast,
    BurnWest,
}

impl VehicleControlPolicy {
    pub fn hold_pos(pos: DVec2, angle: f64) -> Self {
        Self::PositionHold(vec![(pos, angle)])
    }

    pub fn to_status_str(&self) -> String {
        match self {
            VehicleControlPolicy::Idle => "Idling",
            VehicleControlPolicy::External => "Under external control",
            VehicleControlPolicy::PositionHold(_) => "Holding position",
            VehicleControlPolicy::LaunchToOrbit(_) => "Launching to orbit",
            VehicleControlPolicy::BurnPrograde => "Burning prograde",
            VehicleControlPolicy::BurnRetrograde => "Burning retrograde",
            VehicleControlPolicy::HoldAttitude(_) => "Holding attitude",
            VehicleControlPolicy::BurnNorth => "Burning north",
            VehicleControlPolicy::BurnSouth => "Burning south",
            VehicleControlPolicy::BurnEast => "Burning east",
            VehicleControlPolicy::BurnWest => "Burning west",
        }
        .to_string()
    }

    pub fn is_idle(&self) -> bool {
        match self {
            VehicleControlPolicy::Idle => true,
            _ => false,
        }
    }

    pub fn is_prograde(&self) -> bool {
        match self {
            VehicleControlPolicy::BurnPrograde => true,
            _ => false,
        }
    }

    pub fn is_retrograde(&self) -> bool {
        match self {
            VehicleControlPolicy::BurnRetrograde => true,
            _ => false,
        }
    }

    pub fn is_attitude_hold(&self) -> bool {
        match self {
            VehicleControlPolicy::HoldAttitude(_) => true,
            _ => false,
        }
    }

    pub fn is_position_hold(&self) -> bool {
        match self {
            VehicleControlPolicy::PositionHold(_) => true,
            _ => false,
        }
    }

    pub fn is_launch_to_orbit(&self) -> bool {
        match self {
            VehicleControlPolicy::LaunchToOrbit(_) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VehicleController {
    status: VehicleControlStatus,
    mode: VehicleControlPolicy,
}

pub type Pose = (DVec2, f64);

impl VehicleController {
    pub fn idle() -> Self {
        Self {
            status: VehicleControlStatus::Done,
            mode: VehicleControlPolicy::Idle,
        }
    }

    pub fn external() -> Self {
        Self {
            status: VehicleControlStatus::WaitingForInput,
            mode: VehicleControlPolicy::External,
        }
    }

    pub fn launch() -> Self {
        Self {
            status: VehicleControlStatus::InProgress,
            mode: VehicleControlPolicy::LaunchToOrbit(rand(300_000.0, 700_000.0) as f64),
        }
    }

    pub fn position_hold(pose: Pose) -> Self {
        Self {
            status: VehicleControlStatus::InProgress,
            mode: VehicleControlPolicy::PositionHold(vec![pose]),
        }
    }

    pub fn mission(poses: Vec<Pose>) -> Self {
        Self {
            status: VehicleControlStatus::InProgress,
            mode: VehicleControlPolicy::PositionHold(poses),
        }
    }

    pub fn set_policy(&mut self, policy: VehicleControlPolicy) {
        self.mode = policy;
    }

    pub fn set_status(&mut self, status: VehicleControlStatus) {
        self.status = status;
    }

    pub fn status(&self) -> VehicleControlStatus {
        self.status
    }

    pub fn set_idle(&mut self) {
        self.mode = VehicleControlPolicy::Idle;
        self.status = VehicleControlStatus::Idling;
    }

    pub fn enqueue_target_pose(&mut self, pose: Pose, clear_queue: bool) {
        if let VehicleControlPolicy::PositionHold(queue) = &mut self.mode {
            if clear_queue {
                queue.clear();
            }
            queue.push(pose);
        } else {
            self.mode = VehicleControlPolicy::PositionHold(vec![pose]);
        }
    }

    pub fn mode(&self) -> &VehicleControlPolicy {
        &self.mode
    }

    pub fn is_idle(&self) -> bool {
        match self.mode {
            VehicleControlPolicy::Idle => true,
            _ => false,
        }
    }

    pub fn is_attitude_hold(&self) -> bool {
        match self.mode {
            VehicleControlPolicy::HoldAttitude(_) => true,
            _ => false,
        }
    }

    pub fn get_target_pose(&self) -> Option<Pose> {
        self.get_target_queue().next()
    }

    pub fn get_target_queue(&self) -> impl Iterator<Item = Pose> + use<'_> {
        match &self.mode {
            VehicleControlPolicy::PositionHold(queue) => queue.iter().cloned(),
            _ => [].iter().cloned(),
        }
    }

    fn mark_target_achieved(&mut self) {
        if let VehicleControlPolicy::PositionHold(queue) = &mut self.mode {
            if queue.len() > 1 {
                queue.remove(0);
            }
        }
    }

    pub fn check_target_achieved(&mut self, body: &RigidBody, ignore_angle: bool) {
        let (pos, angle) = match self.get_target_pose() {
            Some(p) => p,
            None => return,
        };

        let d = pos.distance(body.pv.pos).abs();
        let v = body.pv.vel.length().abs();
        let a = wrap_pi_npi_f64(angle - body.angle).abs();

        if d > 2.0 {
            return;
        }

        if v > 5.0 {
            return;
        }

        if a > 0.1 && !ignore_angle {
            return;
        }

        self.mark_target_achieved();
    }
}
