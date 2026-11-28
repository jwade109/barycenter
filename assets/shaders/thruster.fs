#version 330

uniform float _current_time;
uniform vec2 _resolution;
uniform vec2 _lower_bound;

float sdf_circle(vec2 center, float radius, vec2 sample) {
  return length(center - sample) - radius;
}

float sdf_diamond(float y, float slope, float width, vec2 sample) {
  float dy = abs(sample.y - y);
  float w = dy * slope + width;
  float cx = _resolution.x / 2.0;
  float dx = abs(sample.x - cx) - w;
  return dx;
}

void main() {

  vec2 coord = gl_FragCoord.xy;
  // coord.x -= _resolution.y / 2;

  coord.yx = coord.xy - coord.x * 0.4;
  coord.xy *= 10;

  // pixelation
  // int n = 50;
  // coord.xy = round(coord.xy / n) * n;


  float len = 3000;
  float r_start = 300;
  float r_end = 40;
  float speed = 1300;
  int spacing = 205;

  float field = 0.0;
  float r_slope = (r_start - r_end) / len;

  float linear_offset = mod(_current_time * speed, spacing);

  for (int i = -2; i < 6; ++i) {
    float strength = 1.0 - smoothstep(len - 300, len, coord.y);
    float extra_width = sin(coord.y / 50) * 5;
    float width = r_start - coord.y * r_slope + extra_width;
    float y = i * spacing + linear_offset;
    field += max(-sdf_diamond(y, -0.3, width * strength, coord), 0);
  }

  gl_FragColor = vec4(0, 0, 0, 0);
  // gl_FragColor = vec4(vec3(step(1, field)), 1);
  gl_FragColor.r = smoothstep(1, 60, field);
  gl_FragColor.g = smoothstep(40, 100, field);
  gl_FragColor.b = smoothstep(140, 190, field);
  gl_FragColor.a = smoothstep(1, 100, field);

  // gl_FragColor.r = 1;
  // gl_FragColor.a = 0.3;
}
