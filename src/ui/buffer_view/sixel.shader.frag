#version 330
precision highp float;

uniform sampler2D   u_render_texture;
uniform vec2        u_resolution;
uniform sampler2D   u_sixel;
uniform vec4        u_sixel_rectangle;

out vec4 color;

void main (void) {
    vec2 view_coord = gl_FragCoord.xy / u_resolution;
    color = texture(u_render_texture, view_coord);
    view_coord = vec2(view_coord.s, 1.0 - view_coord.t);

    vec2 start = u_sixel_rectangle.xy / u_resolution;
    vec2 end = u_sixel_rectangle.zw / u_resolution;
    
    if (start.x <= view_coord.x && start.y <= view_coord.y && view_coord.x < end.x && view_coord.y < end.y) {
        color = texture(u_sixel, vec2((view_coord - start) / (end - start)));
    }
}