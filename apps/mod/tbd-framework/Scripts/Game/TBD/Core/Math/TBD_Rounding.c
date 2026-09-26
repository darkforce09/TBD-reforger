/**
 * @file TBD_Rounding.c
 * @brief Nearest-integer rounding for both signs.
 *
 * Role: float-to-int rounding where truncation would bias a position or a count.
 * Position: called by the task state machine and the marker data builder.  State: none.
 * Invariants: halves round away from zero (2.5 -> 3, -2.5 -> -3); written without a ternary,
 * which Enforce Script does not have.
 */

//! Stateless rounding helpers.
class TBD_Rounding
{
	//! The nearest integer to `value`, halves away from zero.
	//! @param value the float to round
	//! @return the rounded integer; never fails
	static int RoundToInt(float value)
	{
		if (value >= 0)
			return value + 0.5;

		return value - 0.5;
	}
}
