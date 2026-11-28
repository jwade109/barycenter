#version 330

// standard uniforms
uniform float _current_time;
uniform vec2 _resolution;
uniform vec2 _mouse_pos;

// shader-specific uniforms
uniform vec3 _isometry;
uniform float _scale;

float sdf_circle(vec2 center, float radius, vec2 sample) {
  return length(center - sample) - radius;
}

float sdf_diamond(float y, float slope, float width, vec2 sample) {
  float dy = abs(sample.y - y);
  float w = dy * slope + width;
  float dx = abs(sample.x) - w;
  return dx;
}

mat2 rot2d(float angle) {
  float c = cos(angle);
  float s = sin(angle);
  return mat2(c, s, -s, c);
}

void main() {

  vec2 coord = gl_FragCoord.xy;

  if (coord.x < 10 || coord.y < 10 || coord.x > _resolution.x - 10 ||
      coord.y > _resolution.y - 10) {
    gl_FragColor = vec4(0.1, 0.1, 0.5, 0.5);
    return;
  }

  vec2 origin = _mouse_pos; // _isometry.xy;
  float scale = 0.4 + sin(_current_time) * 0.1;
  float angle = _isometry.z;

  coord -= origin;

  coord = coord * rot2d(angle - 1.57);

  vec2 unscaled = coord;

  coord.x /= 0.002 * _scale;
  coord.y /= 0.008 * _scale;

  // pixelation
  // int n = 50;
  // coord.xy = round(coord.xy / n) * n;

  float len = 1800;
  float r_start = 200;
  float r_end = 2000;
  float speed = 2000;
  int spacing = 205;

  float field = 0.0;
  float r_slope = (r_start - r_end) / len;

  float linear_offset = mod(_current_time * speed, spacing);

  if (coord.y >= 0) {
    for (int i = -2; i < 12; ++i) {
      float strength = 1.0 - smoothstep(len - 700, len - 200, coord.y);
      float extra_width = sin(coord.y / 50) * 5;
      float width = r_start - coord.y * r_slope + extra_width;
      float y = i * spacing + linear_offset;
      field += max(-sdf_diamond(y, -0.3, width * strength, coord), 0);
    }
  }

  gl_FragColor = vec4(0, 0, 0, 0);
  gl_FragColor = vec4(vec3(step(1, field)), 1);
  // gl_FragColor.r = smoothstep(1, 60, field);
  // gl_FragColor.g = smoothstep(40, 400, field);
  // gl_FragColor.b = smoothstep(140, 500, field);
  gl_FragColor.a = smoothstep(0, 100, field) * 0.2;
  gl_FragColor.a *= smoothstep(0, 500, 1000 - coord.y);

  float light_sdf = sdf_circle(vec2(0), 0, unscaled);
  gl_FragColor += vec4(1) * (1 - smoothstep(50, 600, light_sdf));
}
