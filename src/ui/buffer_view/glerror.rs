/// Check for OpenGL error and report it using `log::error`.
///
/// Only active in debug builds!
///
/// ``` no_run
/// # let glow_context = todo!();
/// use check_gl_error;
/// check_for_gl_error!(glow_context);
/// check_for_gl_error!(glow_context, "during painting");
/// ```
#[macro_export]
macro_rules! check_gl_error {
    ($gl: expr, $context: literal) => {{
        if cfg!(debug_assertions) {
            $crate::ui::glerror::check_for_gl_error_impl($gl, file!(), line!(), $context)
        }
    }};
}
#[doc(hidden)]
pub fn check_for_gl_error_impl(gl: &glow::Context, file: &str, line: u32, context: &str) {
    use glow::HasContext as _;

    #[allow(unsafe_code)]
    let error_code = unsafe { gl.get_error() };
    if error_code != glow::NO_ERROR {
        let error_str = match error_code {
            glow::INVALID_ENUM => "GL_INVALID_ENUM",
            glow::INVALID_VALUE => "GL_INVALID_VALUE",
            glow::INVALID_OPERATION => "GL_INVALID_OPERATION",
            glow::STACK_OVERFLOW => "GL_STACK_OVERFLOW",
            glow::STACK_UNDERFLOW => "GL_STACK_UNDERFLOW",
            glow::OUT_OF_MEMORY => "GL_OUT_OF_MEMORY",
            glow::INVALID_FRAMEBUFFER_OPERATION => "GL_INVALID_FRAMEBUFFER_OPERATION",
            glow::CONTEXT_LOST => "GL_CONTEXT_LOST",
            0x8031 => "GL_TABLE_TOO_LARGE1",
            0x9242 => "CONTEXT_LOST_WEBGL",
            _ => "<unknown>",
        };

        if context.is_empty() {
            log::error!(
                "GL error, at {}:{}: {} (0x{:X}). Please file a bug at https://github.com/mkrueger/icy_term/issues",
                file,
                line,
                error_str,
                error_code,
            );
        } else {
            log::error!(
                "GL error, at {}:{} ({}): {} (0x{:X}). Please file a bug at https://github.com/mkrueger/icy_term/issues",
                file,
                line,
                context,
                error_str,
                error_code,
            );
        }
    }
}
