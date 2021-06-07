#version 450

layout(location = 0) out vec4 o_Target;

layout(set = 2, binding = 0) uniform MyMaterial_vectors {
    vec3 camera_position;
    vec3 color;
};
layout(set = 3, binding = 0) uniform MyMaterial_floats {
    float model_size;
    float distance_shading_power;
    float distance_shading_constrict;
};
// layout(set = 4, binding = 0) uniform colorscheme {
//     sampler2D colorscheme;
// };
// layout(set = 3, binding = 1) uniform sampler color_scheme_sampler;

layout(location = 0) in vec3 normalVector;
layout(location = 1) in vec3 lightVector;

// https://stackoverflow.com/a/17897228
// All components are in the range [0…1], including hue.
vec3 rgb2hsv(vec3 c)
{
    vec4 K = vec4(0.0, -1.0 / 3.0, 2.0 / 3.0, -1.0);
    vec4 p = mix(vec4(c.bg, K.wz), vec4(c.gb, K.xy), step(c.b, c.g));
    vec4 q = mix(vec4(p.xyw, c.r), vec4(c.r, p.yzx), step(p.x, c.r));

    float d = q.x - min(q.w, q.y);
    float e = 1.0e-10;
    return vec3(abs(q.z + (q.w - q.y) / (6.0 * d + e)), d / (q.x + e), q.x);
}

// All components are in the range [0…1], including hue.
vec3 hsv2rgb(vec3 c)
{
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}


void main() {
    vec3 color = color;
# ifdef MYMATERIAL_TOON_SHADING
    vec3 cl = color;    
    vec3 light = -normalize(lightVector.xyz);    
    float vdn = light.z;
    cl = round(vdn * 5) / 5 * cl;
    color = cl*vdn;
    if (vdn < 0.3)
    {
        color = vec3(0);
    }
# else
    float ambientReflection = 0.3;
    float ambientIntensity = 1;

    float diffuseReflection = 0.3;
    float diffuseIntensity = 1;

    float specularReflection = 0.5;
    float specularIntensity = 1;

    float shininess = 5;

    // Vector to camera
    vec3 pos = vec3(0);
    vec3 v = normalize(camera_position - pos);

    // Vector to light source
    vec3 light_position = camera_position;
    vec3 lm = normalize(light_position - pos);

    // Reflected light vector
    vec3 np = 2 * normalize(dot(lm, normalVector) * normalVector);
    vec3 rm = normalize(np - lm);

    // Light intensity
    float ip = ambientReflection * ambientIntensity + (diffuseReflection * diffuseIntensity * dot(lm, normalVector) + specularReflection * specularIntensity * pow(max(0, min(1, dot(rm, v))), shininess));

    color = ip * color.xyz;
# endif

    // Reduce Value of colour based on distance from camera.    
    float camera_dist = length(camera_position);
    float near_plane = camera_dist - model_size / 2.0 * distance_shading_constrict;
    float far_plane = camera_dist + model_size / 2.0 * distance_shading_constrict;

    // Calculate magnitude of shading.
    float z = abs(gl_FragCoord.z / gl_FragCoord.w / 1);
    float d = 1.0 - min(smoothstep(near_plane, far_plane, z), distance_shading_power);
    color = rgb2hsv(color);

    // Perform shading on channel of choice.
# ifdef MYMATERIAL_DISTANCE_SHADING_CHANNEL_HUE
    color.x = d;
# endif
# ifdef MYMATERIAL_DISTANCE_SHADING_CHANNEL_SATURATION
    color.y *= d;
# endif
# ifdef MYMATERIAL_DISTANCE_SHADING_CHANNEL_VALUE
    color.z *= d;
# endif
    color = hsv2rgb(color);

    o_Target = vec4(color, 1);
}
