use super::script_core::ScriptCore;
use crate::utils::extensions::OptionExt;
use rune::Value;

/// Experimental function caller.
/// No safety is included with this, use at your own risk!
pub struct FNCaller;

#[allow(clippy::too_many_arguments)]
impl FNCaller {
    /// Takes the function pointer and the paramaters in form of a `Vec<i64>`.
    /// If there are more than 10 entries in `params`, this function sends an error message
    /// and returns 0.
    /// If below or equal to 10 entries, the correct `call_xx` function is found and called.
    /// # Design
    /// This could be done via `c_variadic`, but it causes too many undefined behaviors due to
    /// always passing more parameters than needed.
    pub fn call_auto_raw(fn_ptr: i64, params: Vec<i64>) -> i64 {
        let params_len = params.len();
        if params_len > 10 {
            log!("[ERROR] Max amount of parameters reached, keep it below (or eq. to) 10!");
            log!(
                "[ERROR] Got ",
                params_len,
                " parameters, expected 10 or less, returning 0."
            );
            return 0;
        }

        match params_len {
            0 => Self::call(fn_ptr),
            1 => Self::call_00(fn_ptr, params[0]),
            2 => Self::call_01(fn_ptr, params[0], params[1]),
            3 => Self::call_02(fn_ptr, params[0], params[1], params[2]),
            4 => Self::call_03(fn_ptr, params[0], params[1], params[2], params[3]),
            5 => Self::call_04(
                fn_ptr, params[0], params[1], params[2], params[3], params[4],
            ),
            6 => Self::call_05(
                fn_ptr, params[0], params[1], params[2], params[3], params[4], params[5],
            ),
            7 => Self::call_06(
                fn_ptr, params[0], params[1], params[2], params[3], params[4], params[5], params[6],
            ),
            8 => Self::call_07(
                fn_ptr, params[0], params[1], params[2], params[3], params[4], params[5],
                params[6], params[7],
            ),
            9 => Self::call_08(
                fn_ptr, params[0], params[1], params[2], params[3], params[4], params[5],
                params[6], params[7], params[8],
            ),
            10 => Self::call_09(
                fn_ptr, params[0], params[1], params[2], params[3], params[4], params[5],
                params[6], params[7], params[8], params[9],
            ),
            _ => crash!(
                "[ERROR] Parameter count is unchecked. Got ",
                params_len,
                " parameters, expected 10 or less, closing."
            ),
        }
    }

    /// Same as `call_auto_raw`, but takes all values in the vector as a Rune `Value` and turns it
    /// into the native pointer.
    /// This is recommended for when you don't want to get the pointer of each value manually, but
    /// discouraged when you are passing pre-defined pointers into it.
    pub fn call_auto(fn_ptr: i64, params: Vec<Value>) -> i64 {
        let params_len = params.len();
        if params_len > 10 {
            log!("[ERROR] Max amount of parameters reached, keep it below (or eq. to) 10!");
            log!(
                "[ERROR] Got ",
                params_len,
                " parameters, expected 10 or less, returning 0."
            );
            return 0;
        }

        let params: Vec<i64> = params
            .iter()
            .map(|value| {
                ScriptCore::value_as_ptr(value).unwrap_or_crash(zencstr!(
                    "[ERROR] Couldn't get the pointer of \"",
                    format!("{value:?}"),
                    "\"!"
                )) as i64
            })
            .collect();

        Self::call_auto_raw(fn_ptr, params)
    }

    pub fn call(fn_ptr: i64) -> i64 {
        let fn_ptr = fn_ptr as *const i64;
        unsafe {
            std::mem::transmute::<*const i64, extern "system" fn() -> *const i64>(fn_ptr)() as _
        }
    }

    pub fn call_00(fn_ptr: i64, param_0: i64) -> i64 {
        let fn_ptr = fn_ptr as *const i64;
        unsafe {
            std::mem::transmute::<*const i64, extern "system" fn(*const i64) -> *const i64>(fn_ptr)(
                param_0 as _,
            ) as _
        }
    }

    pub fn call_01(fn_ptr: i64, param_0: i64, param_1: i64) -> i64 {
        let fn_ptr = fn_ptr as *const i64;
        unsafe {
            std::mem::transmute::<
                *const i64,
                extern "system" fn(*const i64, *const i64) -> *const i64,
            >(fn_ptr)(param_0 as _, param_1 as _) as _
        }
    }

    pub fn call_02(fn_ptr: i64, param_0: i64, param_1: i64, param_2: i64) -> i64 {
        let fn_ptr = fn_ptr as *const i64;
        unsafe {
            std::mem::transmute::<
                *const i64,
                extern "system" fn(*const i64, *const i64, *const i64) -> *const i64,
            >(fn_ptr)(param_0 as _, param_1 as _, param_2 as _) as _
        }
    }

    pub fn call_03(fn_ptr: i64, param_0: i64, param_1: i64, param_2: i64, param_3: i64) -> i64 {
        let fn_ptr = fn_ptr as *const i64;
        unsafe {
            std::mem::transmute::<
                *const i64,
                extern "system" fn(*const i64, *const i64, *const i64, *const i64) -> *const i64,
            >(fn_ptr)(param_0 as _, param_1 as _, param_2 as _, param_3 as _) as _
        }
    }

    pub fn call_04(
        fn_ptr: i64,
        param_0: i64,
        param_1: i64,
        param_2: i64,
        param_3: i64,
        param_4: i64,
    ) -> i64 {
        let fn_ptr = fn_ptr as *const i64;
        unsafe {
            std::mem::transmute::<
                *const i64,
                extern "system" fn(
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                ) -> *const i64,
            >(fn_ptr)(
                param_0 as _,
                param_1 as _,
                param_2 as _,
                param_3 as _,
                param_4 as _,
            ) as _
        }
    }

    pub fn call_05(
        fn_ptr: i64,
        param_0: i64,
        param_1: i64,
        param_2: i64,
        param_3: i64,
        param_4: i64,
        param_5: i64,
    ) -> i64 {
        let fn_ptr = fn_ptr as *const i64;
        unsafe {
            std::mem::transmute::<
                *const i64,
                extern "system" fn(
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                ) -> *const i64,
            >(fn_ptr)(
                param_0 as _,
                param_1 as _,
                param_2 as _,
                param_3 as _,
                param_4 as _,
                param_5 as _,
            ) as _
        }
    }

    pub fn call_06(
        fn_ptr: i64,
        param_0: i64,
        param_1: i64,
        param_2: i64,
        param_3: i64,
        param_4: i64,
        param_5: i64,
        param_6: i64,
    ) -> i64 {
        let fn_ptr = fn_ptr as *const i64;
        unsafe {
            std::mem::transmute::<
                *const i64,
                extern "system" fn(
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                ) -> *const i64,
            >(fn_ptr)(
                param_0 as _,
                param_1 as _,
                param_2 as _,
                param_3 as _,
                param_4 as _,
                param_5 as _,
                param_6 as _,
            ) as _
        }
    }

    pub fn call_07(
        fn_ptr: i64,
        param_0: i64,
        param_1: i64,
        param_2: i64,
        param_3: i64,
        param_4: i64,
        param_5: i64,
        param_6: i64,
        param_7: i64,
    ) -> i64 {
        let fn_ptr = fn_ptr as *const i64;
        unsafe {
            std::mem::transmute::<
                *const i64,
                extern "system" fn(
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                ) -> *const i64,
            >(fn_ptr)(
                param_0 as _,
                param_1 as _,
                param_2 as _,
                param_3 as _,
                param_4 as _,
                param_5 as _,
                param_6 as _,
                param_7 as _,
            ) as _
        }
    }

    pub fn call_08(
        fn_ptr: i64,
        param_0: i64,
        param_1: i64,
        param_2: i64,
        param_3: i64,
        param_4: i64,
        param_5: i64,
        param_6: i64,
        param_7: i64,
        param_8: i64,
    ) -> i64 {
        let fn_ptr = fn_ptr as *const i64;
        unsafe {
            std::mem::transmute::<
                *const i64,
                extern "system" fn(
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                ) -> *const i64,
            >(fn_ptr)(
                param_0 as _,
                param_1 as _,
                param_2 as _,
                param_3 as _,
                param_4 as _,
                param_5 as _,
                param_6 as _,
                param_7 as _,
                param_8 as _,
            ) as _
        }
    }

    pub fn call_09(
        fn_ptr: i64,
        param_0: i64,
        param_1: i64,
        param_2: i64,
        param_3: i64,
        param_4: i64,
        param_5: i64,
        param_6: i64,
        param_7: i64,
        param_8: i64,
        param_9: i64,
    ) -> i64 {
        let fn_ptr = fn_ptr as *const i64;
        unsafe {
            std::mem::transmute::<
                *const i64,
                extern "system" fn(
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                    *const i64,
                ) -> *const i64,
            >(fn_ptr)(
                param_0 as _,
                param_1 as _,
                param_2 as _,
                param_3 as _,
                param_4 as _,
                param_5 as _,
                param_6 as _,
                param_7 as _,
                param_8 as _,
                param_9 as _,
            ) as _
        }
    }
}
