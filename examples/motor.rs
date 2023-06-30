
use anyhow::Result;
use xipdriver_rs::umv_motor_controller::UmvMotorController;
use std::{thread, time};

fn main() -> Result<()> {
    let hw_json = xipdriver_rs::hwinfo::read("hwinfo.json")?;
    let motor = UmvMotorController::new(&hw_json["/umv_motor_controller_0"])?;

    motor.set_kp(13.)?;
    motor.set_ki(11.)?;
    motor.set_kd(2.)?;
    motor.set_bias(0.)?;
    motor.reset_total_rotation();

    let rpm = motor.get_max_rpm() / 4.;

    let tire_radius: f32 = 3.;
    let target_distance = 10.;
    let sleep_dur = time::Duration::from_millis(50);

    motor.write_brake(false);
    motor.set_accel_rpm(rpm, rpm)?;
    loop {
        let distance = motor.get_total_rotation_left() * 2. * tire_radius * std::f32::consts::PI;
        println!("{} RPM (Read val: {}) distance: {} cm / {} cm",
            motor.get_wheel_rpm_left(),
            motor.read_rotation_left(),
            distance,
            target_distance
        );
        if target_distance <= distance {
            break;
        }
        thread::sleep(sleep_dur);
    }
    motor.write_accel(0, 0)?;
    motor.write_brake(true);

    motor.reset_total_rotation();
    thread::sleep(time::Duration::from_millis(500));

    motor.write_brake(false);
    motor.set_accel_rpm(-rpm, -rpm)?;
    loop {
        let distance = motor.get_total_rotation_left() * 2. * tire_radius * std::f32::consts::PI;
        println!("{} RPM (Read val: {}) distance: {} cm / {} cm",
            motor.get_wheel_rpm_left(),
            motor.read_rotation_left(),
            distance,
            target_distance
        );
        if target_distance <= distance {
            break;
        }
        thread::sleep(sleep_dur);
    }
    motor.write_accel(0, 0)?;
    motor.write_brake(true);

    println!("{}", motor.get_total_rotation_left());
    println!("{}", motor.get_total_rotation_right());

    Ok(())
}
