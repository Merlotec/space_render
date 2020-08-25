#version 450
#extension GL_ARB_separate_shader_objects : enable


const float PI = 3.14159265359;
const float MAX = 10000.0;

const uint MAX_PLANETS = 8;
const uint MAX_STARS = 4;

struct PlanetData {
    vec3 center;
    float radius;
    vec3 hue;
    float atmosphere_radius;
    float atmosphere_density;
};

struct StarData {
    vec3 center;
    float radius;
    vec3 color;
};

layout(std140, set = 1, binding = 0) uniform PlanetList {
    uint planet_count;
    PlanetData planets[MAX_PLANETS];
};

layout(std140, set = 2, binding = 0) uniform Stars {
    uint star_count;
    StarData[MAX_STARS] stars;
};

layout(location = 0) in vec2 ndc;
layout(location = 1) flat in mat4 view;
layout(location = 5) flat in mat4 inv_proj;
layout(location = 9) flat in mat4 proj;

layout(location = 0) out vec4 target;

// ray intersects sphere
// e = -b +/- sqrt( b^2 - c )
vec2 ray_vs_sphere( vec3 p, vec3 dir, float r ) {
    float b = dot( p, dir );
    float c = dot( p, p ) - r * r;

    float d = b * b - c;
    if ( d < 0.0 ) {
        return vec2( MAX, -MAX );
    }
    d = sqrt( d );

    return vec2( -b - d, -b + d );
}

// Mie
// g : ( -0.75, -0.999 )
//      3 * ( 1 - g^2 )               1 + c^2
// F = ----------------- * -------------------------------
//      8pi * ( 2 + g^2 )     ( 1 + g^2 - 2 * g * c )^(3/2)
float phase_mie( float g, float c, float cc ) {
    float gg = g * g;

    float a = ( 1.0 - gg ) * ( 1.0 + cc );

    float b = 1.0 + gg - 2.0 * g * c;
    b *= sqrt( b );
    b *= 2.0 + gg;

    return ( 3.0 / 8.0 / PI ) * a / b;
}

// Rayleigh
// g : 0
// F = 3/16PI * ( 1 + c^2 )
float phase_ray( float cc ) {
    return ( 3.0 / 16.0 / PI ) * ( 1.0 + cc );
}

// scatter const
const float R_INNER = 1.0;
const float R = R_INNER + 0.5;

const int NUM_OUT_SCATTER = 4;
const int NUM_IN_SCATTER = 40;

float density( vec3 p, float ph ) {
    return exp( -max( length( p ) - R_INNER, 0.0 ) / ph );
}

float optic( vec3 p, vec3 q, float ph ) {
    vec3 s = ( q - p ) / float( NUM_OUT_SCATTER );
    vec3 v = p + s * 0.5;

    float sum = 0.0;
    for ( int i = 0; i < NUM_OUT_SCATTER; i++ ) {
        sum += density( v, ph );
        v += s;
    }
    sum *= length( s );

    return sum;
}

vec4 in_scatter( vec3 o, vec3 dir, vec2 e, vec3 l, float ar ) {
    const float rf = ar * 0.85;
    const float ph_ray = 0.01 * rf;
    const float ph_mie = 0.004 * rf;

    const vec4 k_ray = vec4( 3.8, 13.5, 33.1, 10.0 );
    const vec4 k_mie = vec4( 21.0 );
    const float k_mie_ex = 1.1;

    vec4 sum_ray = vec4( 0.0 );
    vec4 sum_mie = vec4( 0.0 );

    float n_ray0 = 0.0;
    float n_mie0 = 0.0;

    float len = ( e.y - e.x ) / float( NUM_IN_SCATTER );
    vec3 s = dir * len;
    vec3 v = o + dir * ( e.x + len * 0.5 );

    for ( int i = 0; i < NUM_IN_SCATTER; i++, v += s ) {
        float d_ray = density( v, ph_ray ) * len;
        float d_mie = density( v, ph_mie ) * len;

        n_ray0 += d_ray;
        n_mie0 += d_mie;

        #if 0
        vec2 e = ray_vs_sphere( v, l, R_INNER );
        e.x = max( e.x, 0.0 );
        if ( e.x < e.y ) {
            continue;
        }
            #endif

        vec2 f = ray_vs_sphere( v, l, ar );
        vec3 u = v + l * f.y;

        float n_ray1 = optic( v, u, ph_ray );
        float n_mie1 = optic( v, u, ph_mie );

        vec4 att = exp( - ( n_ray0 + n_ray1 ) * k_ray - ( n_mie0 + n_mie1 ) * k_mie * k_mie_ex );

        sum_ray += d_ray * att;
        sum_mie += d_mie * att;
    }

    float c  = dot( dir, -l );
    float cc = c * c;
    vec4 scatter =
    sum_ray * k_ray * phase_ray( cc ) +
    sum_mie * k_mie * phase_mie( -0.78, c, cc );


    return scatter;
}

const float DEPTH_PADDING = 0.9;

void main()
{
    target = vec4( 0.0 );
    gl_FragDepth = 1.0;

    if (planet_count <= 0) return;

    vec4 csp = inv_proj * vec4(ndc, 0.5, 1.0);
    vec3 dir = normalize(csp.xyz);

    for (uint p = 0; p < planet_count; p++) {
        // The factor which scales down to 'normalized' scale (planet radius of 1.0).
        const float gk = 1.0 / planets[p].radius;
        // The relative atmosphere radius.
        const float ar = planets[p].atmosphere_radius / planets[p].radius;
        vec3 eye = -(view * vec4(planets[p].center, 1)).xyz;
        //eye *= gk;

        if (dot(eye, vec3(0, 0, 1)) < 0.0) {
            continue;
        }
        vec2 e = ray_vs_sphere( eye, dir, planets[p].atmosphere_radius );
        if ( e.x > e.y ) {
            continue;
        }
        vec2 f = ray_vs_sphere( eye, dir, planets[p].radius );
        e.y = min( e.y, f.x );
        // We need to apply padding to ensure that the atmosphere frag depth is sufficiently greater than the planet mesh depth.
        vec4 world_ndc = (proj * vec4(dir * e.y * DEPTH_PADDING, 1.0));
        gl_FragDepth = world_ndc.z / world_ndc.w;

        // Apply scale down.
        e *= gk;
        eye *= gk;
        for (uint s = 0; s < star_count; s++) {
            vec3 starpos = (view * vec4(stars[s].center, 1)).xyz;
            vec3 l = normalize(starpos - planets[p].center);
            vec4 I = in_scatter(eye, dir, e, l, ar) * planets[p].atmosphere_density;
            vec4 c = pow(I, vec4(1.0/2.2));
            target = c;
            return;
        }
    }
    target = vec4(0);
}
