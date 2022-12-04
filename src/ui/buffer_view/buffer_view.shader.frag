#version 330
precision mediump float;
uniform sampler2DArray u_fonts;
uniform sampler2D u_palette;
uniform sampler2DArray u_buffer;
// uniform sampler2D u_sixel;
            
uniform vec2        u_resolution;
uniform vec2        u_position;
uniform vec2        u_terminal_size;
uniform vec4        u_caret_position;
// uniform vec4        u_sixel_rectangle;

out     vec4        fragColor;

vec4 get_char(vec2 p, float c, float page) {
    if (p.x < 0.|| p.x > 1. || p.y < 0.|| p.y > 1.) {
        return vec4(0, 0, 0, 1.0);
    }
    vec2 v = p / 16.0 + fract(vec2(c, floor(c / 16.0)) / 16.0);
    return textureGrad(u_fonts, vec3(v, page), dFdx(p / 16.0), dFdy(p / 16.0));
}
/* 
// TODO: Is there an easier way to disable the 'intelligent' color conversion crap?
vec4 toLinearSlow(vec4 col) {
    bvec4 cutoff = lessThan(col, vec4(0.04045));
    vec4 higher = vec4(pow((col.rgb + vec3(0.055))/vec3(1.055), vec3(2.4)), col.a);
    vec4 lower = vec4(col.rgb/vec3(12.92), col.a);
    return mix(higher, lower, cutoff);
}*/
            
vec4 get_palette_color(float c) {
    return texture(u_palette, vec2(c, 0));
}

bool check_bit(float v, int bit) {
    return (int(255.0 * v) & (1 << bit)) != 0;
}

void main (void) {
    vec2 view_coord = (gl_FragCoord.xy - u_position) / u_resolution;
    view_coord = vec2(view_coord.s, 1.0 - view_coord.t);

    vec2 sz = u_terminal_size;
    vec2 fb_pos = (view_coord * sz);

    vec4 ch = texture(u_buffer, vec3(view_coord.x, view_coord.y, 0.0));
    vec4 ch_attr = texture(u_buffer, vec3(view_coord.x, view_coord.y, 1.0));
    
    vec2 fract_fb_pos = fract(vec2(fb_pos.x, fb_pos.y));
    vec4 char_data = get_char(fract_fb_pos, ch.x * 255.0, ch.a);
    
    if (char_data.x > 0.5 && !check_bit(ch_attr.r, 7) && !check_bit(ch_attr.r, 4)) {
        fragColor = get_palette_color(ch.y);
    } else {
        fragColor = get_palette_color(ch.z);
    }
/*
    if (u_sixel_rectangle.z > 0.0) {
        vec2 start = (u_sixel_rectangle.xy - u_position) / u_resolution;
        vec2 end = (u_sixel_rectangle.zw - u_position) / u_resolution;
        if (start.x <= view_coord.x && start.y <= view_coord.y && view_coord.x < end.x && view_coord.y < end.y) {
            fragColor = texture(u_sixel, vec2((view_coord - start) / (end - start)));
        }
    }*/

    if (u_caret_position.z > 0 && floor(fb_pos) == u_caret_position.xy) {
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