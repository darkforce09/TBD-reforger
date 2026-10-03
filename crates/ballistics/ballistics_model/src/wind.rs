//! Constant surface wind for the shell flight model.
//!
//! **Role:** turns a meteorological wind report (speed, and the compass direction the wind blows
//! *from*) into the air-velocity vector the flight model subtracts from the shell velocity.
//!
//! **Position:** the `ballistics_model` crate; a fire mission's manual wind entry feeds it, and
//! [`crate::flight_model`] consumes [`Wind::air_velocity_m_s`].
//!
//! **Signals & state:** none; pure value type and functions.
//!
//! **Invariants:** map frame metres, x east, y north, z up; the wind is constant with height, so
//! its vertical component is always zero; air velocity `w = -speed · (sin from, cos from, 0)`;
//! the trigonometry goes through `libm`, so native and WASM builds produce the same bits.

use thiserror::Error;

/// A constant horizontal wind, reported the meteorological way.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wind {
    /// Wind speed in metres per second; zero or more.
    pub speed_m_s: f64,
    /// Compass direction the wind blows from, degrees clockwise from north.
    pub from_deg: f64,
}

/// Why a [`Wind`] report cannot drive the flight model.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum WindError {
    /// The speed is negative, NaN or infinite.
    #[error("wind speed must be a finite number of metres per second, zero or more (got {0})")]
    InvalidSpeed(f64),
    /// The direction is NaN or infinite.
    #[error("wind direction must be a finite number of degrees (got {0})")]
    InvalidDirection(f64),
}

impl Wind {
    /// No wind: zero speed, blowing from north.
    pub const CALM: Wind = Wind {
        speed_m_s: 0.0,
        from_deg: 0.0,
    };

    /// Checks that the speed is finite and not negative and the direction is finite.
    ///
    /// # Errors
    ///
    /// [`WindError::InvalidSpeed`] or [`WindError::InvalidDirection`] naming the offending value.
    pub fn validate(&self) -> Result<(), WindError> {
        if !self.speed_m_s.is_finite() || self.speed_m_s < 0.0 {
            return Err(WindError::InvalidSpeed(self.speed_m_s));
        }
        if !self.from_deg.is_finite() {
            return Err(WindError::InvalidDirection(self.from_deg));
        }
        Ok(())
    }

    /// The velocity of the air in the map frame, metres per second `[east, north, up]`.
    ///
    /// A wind from north (`from_deg = 0`) moves the air south, so the vector points along
    /// negative y; the vertical component is always zero.
    pub fn air_velocity_m_s(&self) -> [f64; 3] {
        let from_rad = self.from_deg.to_radians();
        [
            -self.speed_m_s * libm::sin(from_rad),
            -self.speed_m_s * libm::cos(from_rad),
            0.0,
        ]
    }
}

#[cfg(test)]
#[path = "tests/wind.rs"]
mod tests;
