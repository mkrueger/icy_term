#version 330
in vec2 UV;

uniform sampler2D u_render_texture;
uniform vec2      u_resolution;
uniform vec2      u_position;
uniform float     u_effect;

out vec3 color;

// Scanline CRT shader from
// From https://www.shadertoy.com/view/4scSR8
//
// PUBLIC DOMAIN CRT STYLED SCAN-LINE SHADER
//
//	 by Timothy Lottes
//
// This is more along the style of a really good CGA arcade monitor.
// With RGB inputs instead of NTSC.
// The shadow mask example has the mask rotated 90 degrees for less chromatic aberration.
//
// Left it unoptimized to show the theory behind the algorithm.
//
// It is an example what I personally would want as a display option for pixel art games.
// Please take and use, change, or whatever.
//

// Hardness of scanline.
//	-8.0 = soft
// -16.0 = medium
float sHardScan = -8.0;

// Hardness of pixels in scanline.
// -2.0 = soft
// -4.0 = hard
float kHardPix = -3.0;

// Display warp.
// 0.0 = none
// 1.0 / 8.0 = extreme
vec2 kWarp = vec2(1.0 / 32.0, 1.0 / 24.0);
//const vec2 kWarp = vec2(0);

// Amount of shadow mask.
float kMaskDark = 0.5;
float kMaskLight = 1.5;

//------------------------------------------------------------------------

// sRGB to Linear.
// Assuing using sRGB typed textures this should not be needed.
float toLinear1(float c) {
	return (c <= 0.04045) ?
		(c / 12.92) :
		pow((c + 0.055) / 1.055, 2.4);
}
vec3 toLinear(vec3 c) {
	return vec3(toLinear1(c.r), toLinear1(c.g), toLinear1(c.b));
}

// Linear to sRGB.
// Assuing using sRGB typed textures this should not be needed.
float toSrgb1(float c) {
	return(c < 0.0031308 ?
		(c * 12.92) :
		(1.055 * pow(c, 0.41666) - 0.055));
}
vec3 toSrgb(vec3 c) {
	return vec3(toSrgb1(c.r), toSrgb1(c.g), toSrgb1(c.b));
}

// Nearest emulated sample given floating point position and texel offset.
// Also zero's off screen.
vec4 fetch(vec2 pos, vec2 off)
{
	pos = floor(pos * u_resolution + off) / u_resolution;
	if (max(abs(pos.x - 0.5), abs(pos.y - 0.5)) > 0.5)
		return vec4(vec3(0.0), 0.0);
   	
    vec4 sampledColor = texture(u_render_texture, pos.xy, -16.0);
    /*
    sampledColor = vec4(
        (sampledColor.rgb * sampledColor.a) +
        	(kBackgroundColor * (1.0 - sampledColor.a)),
        1.0
    );*/
    
	return vec4(
        toLinear(sampledColor.rgb),
        sampledColor.a
    );
}

// Distance in emulated pixels to nearest texel.
vec2 dist(vec2 pos) {
	pos = pos * u_resolution;
	return -((pos - floor(pos)) - vec2(0.5));
}

// 1D Gaussian.
float gaus(float pos, float scale) {
	return exp2(scale * pos * pos);
}

// 3-tap Gaussian filter along horz line.
vec3 horz3(vec2 pos, float off)
{
	vec3 b = fetch(pos, vec2(-1.0, off)).rgb;
	vec3 c = fetch(pos, vec2( 0.0, off)).rgb;
	vec3 d = fetch(pos, vec2(+1.0, off)).rgb;
	float dst = dist(pos).x;
	// Convert distance to weight.
	float scale = kHardPix;
	float wb = gaus(dst - 1.0, scale);
	float wc = gaus(dst + 0.0, scale);
	float wd = gaus(dst + 1.0, scale);
	// Return filtered sample.
	return (b * wb + c * wc + d * wd) / (wb + wc + wd);
}

// 5-tap Gaussian filter along horz line.
vec3 horz5(vec2 pos, float off)
{
	vec3 a = fetch(pos, vec2(-2.0, off)).rgb;
	vec3 b = fetch(pos, vec2(-1.0, off)).rgb;
	vec3 c = fetch(pos, vec2( 0.0, off)).rgb;
	vec3 d = fetch(pos, vec2(+1.0, off)).rgb;
	vec3 e = fetch(pos, vec2(+2.0, off)).rgb;
	float dst = dist(pos).x;
	// Convert distance to weight.
	float scale = kHardPix;
	float wa = gaus(dst - 2.0, scale);
	float wb = gaus(dst - 1.0, scale);
	float wc = gaus(dst + 0.0, scale);
	float wd = gaus(dst + 1.0, scale);
	float we = gaus(dst + 2.0, scale);
	// Return filtered sample.
	return (a * wa + b * wb + c * wc + d * wd + e * we) / (wa + wb + wc + wd + we);
}

// Return scanline weight.
float scan(vec2 pos, float off) {
	float dst = dist(pos).y;
	return gaus(dst + off, sHardScan);
}

// Allow nearest three lines to effect pixel.
vec3 tri(vec2 pos)
{
	vec3 a = horz3(pos, -1.0);
	vec3 b = horz5(pos,  0.0);
	vec3 c = horz3(pos, +1.0);
	float wa = scan(pos, -1.0);
	float wb = scan(pos,  0.0);
	float wc = scan(pos, +1.0);
	return a * wa + b * wb + c * wc;}

// Distortion of scanlines, and end of screen alpha.
vec2 warp(vec2 pos)
{
	pos = pos * 2.0 - 1.0;
	pos *= vec2(
		1.0 + (pos.y * pos.y) * kWarp.x,
		1.0 + (pos.x * pos.x) * kWarp.y
	);
	return pos * 0.5 + 0.5;
}

// Shadow mask.
vec3 mask(vec2 pos)
{
	pos.x += pos.y * 3.0;
	vec3 mask = vec3(kMaskDark, kMaskDark, kMaskDark);
	pos.x = fract(pos.x / 6.0);
	if (pos.x < 0.333)
		mask.r = kMaskLight;
	else if (pos.x < 0.666)
		mask.g = kMaskLight;
	else
		mask.b = kMaskLight;
	return mask;
}

float rand(vec2 co) {
	return fract(sin(dot(co.xy , vec2(12.9898, 78.233))) * 43758.5453);
}

void scanlines1(bool curved)
{
    vec2 fragCoord = (gl_FragCoord.xy - u_position);
    vec2 pos = curved ? warp(fragCoord / u_resolution) : fragCoord / u_resolution;
    vec4 unmodifiedColor = fetch(pos, vec2(0));
    color.rgb = tri(pos) * mask(fragCoord.xy);
	color = toSrgb(color.rgb);
}
// Effect 2
// https://www.shadertoy.com/view/XdyGzR
#define CURVATURE 1.
#define SCANLINES 1.
#define CURVED_SCANLINES 1.
#define BLURED 1.
#define LIGHT 1.
#define COLOR_CORRECTION 1.
//#define ASPECT_RATIO 1.

const float gamma = 1.;
const float contrast = 1.;
const float saturation = 1.;
const float brightness = 1.;

const float light = 9.;
const float blur = 1.5;

vec3 postEffects(in vec3 rgb, in vec2 xy) {
    rgb = pow(rgb, vec3(gamma));
    rgb = mix(vec3(.5), mix(vec3(dot(vec3(.2125, .7154, .0721), rgb*brightness)), rgb*brightness, saturation), contrast);

    return rgb;
}

// Sigma 1. Size 3
vec3 gaussian(in vec2 uv) {
    float b = blur / (u_resolution.x / u_resolution.y);

    uv+= .5;

    vec3 col = texture(u_render_texture, vec2(uv.x - b/u_resolution.x, uv.y - b/u_resolution.y) ).rgb * 0.077847;
    col += texture(u_render_texture, vec2(uv.x - b/u_resolution.x, uv.y) ).rgb * 0.123317;
    col += texture(u_render_texture, vec2(uv.x - b/u_resolution.x, uv.y + b/u_resolution.y) ).rgb * 0.077847;

    col += texture(u_render_texture, vec2(uv.x, uv.y - b/u_resolution.y) ).rgb * 0.123317;
    col += texture(u_render_texture, vec2(uv.x, uv.y) ).rgb * 0.195346;
    col += texture(u_render_texture, vec2(uv.x, uv.y + b/u_resolution.y) ).rgb * 0.123317;

    col += texture(u_render_texture, vec2(uv.x + b/u_resolution.x, uv.y - b/u_resolution.y) ).rgb * 0.077847;
    col += texture(u_render_texture, vec2(uv.x + b/u_resolution.x, uv.y) ).rgb * 0.123317;
    col += texture(u_render_texture, vec2(uv.x + b/u_resolution.x, uv.y + b/u_resolution.y) ).rgb * 0.077847;

    return col;
}

void scanlines2(bool curvature, bool blured, bool curved_scanlines, bool scanlines, float light, bool color_correction)
{
	vec2 st = ((gl_FragCoord.xy - u_position) / u_resolution.xy) - vec2(.5);
    // Curvature/light
    float d = length(st*.5 * st*.5);
    vec2 uv = curvature ? st*d + st*.935 : st;

    // Fudge aspect ratio
#ifdef ASPECT_RATIO
    uv.x *= u_resolution.x/u_resolution.y*.75;
#endif
    
    // CRT color blur
    vec3 col = blured ? gaussian(uv) : texture(u_render_texture, uv+.5).rgb;

    // Light
	if (light > 0.0) {
    	float l = 1. - min(1., d*light);
    	col *= l;
	}

    // Scanlines
    float y = curved_scanlines ? uv.y : st.y;

    float showScanlines = 1.;
    if (u_resolution.y<360.) showScanlines = 0.;
    
	if (scanlines) {
		float s = 1. - smoothstep(320., 1440., u_resolution.y) + 1.;
		float j = cos(y*u_resolution.y*s)*.1; // values between .01 to .25 are ok.
		col = abs(showScanlines-1.)*col + showScanlines*(col - col*j);
		col *= 1. - ( .01 + ceil(mod( (st.x+.5)*u_resolution.x, 3.) ) * (.995-1.01) )*showScanlines;
	}
    // Border mask
	if (curvature) {
        float m = max(0.0, 1. - 2.*max(abs(uv.x), abs(uv.y) ) );
        m = min(m*200., 1.);
        col *= m;
	}

    // Color correction
    color = color_correction ? postEffects(col, st) : max(vec3(.0), min(vec3(1.), col));
}

void main() {
    if (u_effect < 1.0) { 
        vec2 uv = (gl_FragCoord.xy - u_position) / u_resolution;
        color = texture(u_render_texture, uv).xyz;
    } else if (u_effect < 2.0) { 
        scanlines1(false);
    }  else if (u_effect < 3.0) { 
        scanlines1(true);
    } else if (u_effect < 4.0) { 
        scanlines2(false, true, true, true, 1.0, false);
    } else {
        scanlines2(true, true, true, true, 1.0, true);
    }
}