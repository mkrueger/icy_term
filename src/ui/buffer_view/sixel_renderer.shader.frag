#version 300 es

precision highp float;

uniform sampler2D   u_render_texture;
uniform vec2        u_resolution;
uniform sampler2D   u_sixel;
uniform vec4        u_sixel_rectangle;

out vec4 color;

void main (void) {
    vec2 view_coord = gl_FragCoord.xy / u_resolution;
    vec2 flipped_view_coord = vec2(view_coord.s, 1.0 - view_coord.t);

    vec2 upper_left = u_sixel_rectangle.xy / u_resolution;
    vec2 bottom_right = u_sixel_rectangle.zw / u_resolution;
    
    if (upper_left.x <= flipped_view_coord.x && 
        upper_left.y <= flipped_view_coord.y && 
        flipped_view_coord.x < bottom_right.x && 
        flipped_view_coord.y < bottom_right.y) {
        color = texture(u_sixel, vec2((flipped_view_coord - upper_left) / (bottom_right - upper_left)));
    } else {
        color = texture(u_render_texture, view_coord);
    }
}