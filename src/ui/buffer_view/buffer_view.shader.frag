#version 330
precision highp float;

uniform sampler2DArray u_fonts;
uniform sampler2D u_palette;
uniform sampler2DArray u_buffer;
uniform sampler2D u_sixel;
            
uniform vec2        u_resolution;
uniform vec2        u_position;
uniform vec2        u_terminal_size;
uniform vec4        u_caret_position;
uniform vec4        u_sixel_rectangle;

uniform vec4        u_selection;
uniform float       u_selection_attr;

uniform float       u_blink;

out     vec4        fragColor;

vec4 get_char(vec2 p, float c, float page) {
    if (p.x < 0.|| p.x > 1. || p.y < 0.|| p.y > 1.) {
        return vec4(0, 0, 0, 1.0);
    }
    vec2 v = p / 16.0 + fract(vec2(c, floor(c / 16.0)) / 16.0);
    return textureGrad(u_fonts, vec3(v, page), dFdx(p / 16.0), dFdy(p / 16.0));
}
            
vec4 get_palette_color(float c) {
    return texture(u_palette, vec2(c, 0));
}

bool check_bit(float v, int bit) {
    return (int(255.0 * v) & (1 << bit)) != 0;
}

bool is_selected(float x, float y) {
    if (u_selection_attr > 0.0) {
        return u_selection.y <= y && y <= u_selection.w && u_selection.x <= x && x <= u_selection.z;
    }
    return u_selection.y == y && u_selection.w == y && u_selection.x <= x && x <= u_selection.z || // same line
           u_selection.y < y && y < u_selection.w || // between start & end line
           u_selection.y == y && u_selection.w != y && u_selection.x <= x  || // start line
           u_selection.w == y && u_selection.y != y && x <= u_selection.z;    // end line
}

void main (void) {
    vec2 view_coord = (gl_FragCoord.xy - u_position) / u_resolution;
    view_coord = vec2(view_coord.s, 1.0 - view_coord.t);

    vec2 sz = u_terminal_size;
    vec2 fb_pos = (view_coord * sz);

    vec4 ch = texture(u_buffer, vec3(view_coord.x, view_coord.y, 0.0));
    vec4 ch_attr = texture(u_buffer, vec3(view_coord.x, view_coord.y, 1.0));
    
    vec2 fract_fb_pos = fract(vec2(fb_pos.x, fb_pos.y));

    float ch_value = ch.x * 255.0;
    // double height
    if (check_bit(ch_attr[0], 3)) {
        fract_fb_pos.y /= 2.0;
        // 2nd line
        if (check_bit(ch_attr[0], 4)) {
            fract_fb_pos.y += 0.5;
        }
    }

    vec4 char_data = get_char(fract_fb_pos, ch_value, ch.a);
    
    vec4 fg = get_palette_color(ch.y);
    vec4 bg = get_palette_color(ch.z);

    if (u_selection_attr >= 0.0) {
        float x = floor(fb_pos.x);
        float y = floor(fb_pos.y);
        if (is_selected(x, y)) {
            vec4 tmp = bg;
            bg = fg;
            fg = tmp;
        }
    }

    if (char_data.x > 0.5 && (ch_attr[3] == 0.0 || u_blink > 0)) {
        fragColor = fg;
    } else {
        fragColor = bg;
    }

    // underline
    if (check_bit(ch_attr[0], 0)) {
        if (fract_fb_pos.y >= 15.0 / 16.0) {
            fragColor = fg;
        }
    }

    // double underline
    if (check_bit(ch_attr[0], 1)) {
        if (fract_fb_pos.y >= 13.0 / 16.0 && fract_fb_pos.y < 14.0 / 16.0) {
            fragColor = fg;
        }
    }

    // strike through
    if (check_bit(ch_attr[0], 2)) {
        if (fract_fb_pos.y >= 7.0 / 16.0 && fract_fb_pos.y < 8.0 / 16.0) {
            fragColor = fg;
        }
    }

    if (u_sixel_rectangle.z > 0.0) {
        vec2 start = (u_sixel_rectangle.xy ) / u_resolution;
        vec2 end = (u_sixel_rectangle.zw ) / u_resolution;
        if (start.x <= view_coord.x && start.y <= view_coord.y && view_coord.x < end.x && view_coord.y < end.y) {
            fragColor = texture(u_sixel, vec2((view_coord - start) / (end - start)));
        }
    }

    if (u_selection_attr < 0.0 && u_caret_position.z > 0 && floor(fb_pos) == u_caret_position.xy) {
        if (u_caret_position.w == 0.0) { // underscore
            if (fract_fb_pos.y >= 13.0 / 16.0 && fract_fb_pos.y <= 15.0 / 16.0) {
                fragColor = get_palette_color(ch.y);
            }
        } else if (u_caret_position.w == 1.0) { // half height
            if (fract_fb_pos.y >= 0.5) {
                fragColor = get_palette_color(ch.y);
            }
        }
    }
}