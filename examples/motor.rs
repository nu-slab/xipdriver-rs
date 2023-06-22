
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

    motor.write_brake(false);
    motor.write_accel_left(80)?;
    for _ in 0..10 {
        println!("{} RPM (Read val: {})", motor.get_wheel_rpm_left(), motor.read_rotation_left());
        thread::sleep(time::Duration::from_millis(500));
    }
    motor.write_accel_left(0)?;
    motor.write_brake(true);

    println!("{}", motor.get_total_rotation_left());
    println!("{}", motor.get_total_rotation_right());

    Ok(())
}
