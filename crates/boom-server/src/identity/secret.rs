//! `Secret<T>` — bọc giá trị nhạy cảm (token phiên, PHC string) theo ADR 0011 §6.
//!
//! Quy tắc (ADR 0011 §6 "Không rò token trong log"):
//!
//! - `Debug` in **dấu ẩn**, không in giá trị — nên `{:?}` trong log/assert là an toàn.
//! - **Không** có `Display`: muốn in ra log thì phải gọi [`Secret::expose`] có chủ đích,
//!   để reviewer thấy chỗ lộ giá trị.
//! - Không `Clone` mặc định: nhân bản bí mật là thao tác phải được cân nhắc, không tự động.
//!
//! ```
//! use boom_server::identity::Secret;
//!
//! let secret = Secret::new("token-thao");
//! assert_eq!(format!("{secret:?}"), "Secret(<hidden>)");
//! assert_eq!(*secret.expose(), "token-thao");
//! ```

use std::fmt;

/// Bọc giá trị nhạy cảm. Xem [module](self).
#[derive(Default, PartialEq, Eq)]
pub struct Secret<T> {
    inner: T,
}

impl<T> Secret<T> {
    /// Bọc giá trị nhạy cảm.
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Lấy tham chiếu tới giá trị bên trong.
    ///
    /// Dùng khi thật sự cần giá trị (ví dụ bind tham số SQL). Mọi chỗ gọi `expose` nên nằm
    /// trong tầng lưu trữ, không nằm trong log.
    pub fn expose(&self) -> &T {
        &self.inner
    }

    /// Lấy giá trị ra khỏi wrapper (vd khi tiêu thụ xong).
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> fmt::Debug for Secret<T> {
    /// In dấu ẩn, **không** in giá trị — kể cả khi `T` có `Debug` (ADR 0011 §6).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<hidden>)")
    }
}

#[cfg(test)]
mod tests {
    use super::Secret;

    #[test]
    fn debug_does_not_print_value() {
        let secret = Secret::new("hunter2-token");
        let text = format!("{secret:?}");
        assert_eq!(text, "Secret(<hidden>)");
        assert!(!text.contains("hunter2"));
        assert!(!text.contains("token"));
    }

    #[test]
    fn debug_of_holder_does_not_leak_inner_value() {
        #[derive(Debug)]
        struct Holder {
            password_hash: Secret<String>,
        }

        let holder = Holder {
            password_hash: Secret::new("s3cr3t-phc-string".to_string()),
        };
        let text = format!("{holder:?}");
        assert!(!text.contains("s3cr3t"));
        assert!(text.contains("Secret(<hidden>)"));
        // Giá trị vẫn lấy được qua `expose` có chủ đích (không qua log).
        assert_eq!(holder.password_hash.expose(), "s3cr3t-phc-string");
    }

    #[test]
    fn expose_and_into_inner_return_value() {
        let secret = Secret::new(String::from("abc"));
        assert_eq!(secret.expose(), "abc");
        assert_eq!(secret.into_inner(), String::from("abc"));
    }
}
