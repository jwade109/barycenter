#version 330

// Input vertex attributes (from vertex shader)
in vec2 fragTexCoord;
in vec4 fragColor;

varying vec2 vUv;

// Input uniform values
uniform sampler2D texture0;
uniform vec4 colDiffuse;

uniform float seconds;

uniform vec2 screen_size;

uniform float freqX;
uniform float freqY;
uniform float ampX;
uniform float ampY;
uniform float speedX;
uniform float speedY;

float sdf_circle(vec2 center, float radius, vec2 sample) {
  return length(center - sample) - radius;
}

float sdf_diamond(float y, float slope, float width, vec2 sample) {
  float dy = abs(sample.y - y);
  float w = dy * slope + width;
  float cx = screen_size.x / 2.0;
  float dx = abs(sample.x - cx) - w;
  return dx;
}

void main() {

  // vec2 uv = coord / screen_size;

  vec2 coord = gl_FragCoord.xy;

  float d_min = 10000;

  float r_slope = 0.1; // + sin(coord.y / 30 - seconds * 12) * 0.01;
  float r_start = 20;
  float speed = 2000; //  + sin(seconds / 5) * 60;
  int spacing = 220;

  float field = 0.0;

  float offset = mod(seconds * speed, spacing);

  for (int i = -2; i < 12; ++i) {

    float extra_width = sin(coord.y / 50 * (1.0 + sin(seconds))) * 5;
    float width = 150 - coord.y * r_slope + extra_width;
    float y = i * spacing + offset;
    field += max(-sdf_diamond(y, -0.3, width, coord), 0);
  }

  gl_FragColor = vec4(0.3, 0.3, 1.0, 1.0);

  // gl_FragColor.x = smoothstep(30, 40, field);
  // gl_FragColor.y = smoothstep(10, 20, field);
  gl_FragColor.r = smoothstep(1, 60, field);
  gl_FragColor.g = smoothstep(40, 100, field);
  gl_FragColor.b = smoothstep(50, 200, field);

  // for (int i = -2; i < 12; ++i) {
  //   float y = i * spacing;
  //   y += (int((seconds * speed))) % spacing;
  //   float r = r_start + y * r_slope;
  //   vec2 center = vec2(screen_size.x / 2, y);
  //   vec2 delta = coord - center;
  //   delta.x *= 4;
  //   float d = sdf_circle(delta, r, vec2(0, 0));
  //   d_min = min(d, d_min);
  // }

  // vec2 bl = vec2(screen_size.x / 2, 0);

  // d_min = max(d_min, sdf_circle(coord, 900, bl));

  // gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0);

  // gl_FragColor.r = 1 - step(0, d_min);
  // gl_FragColor.g = 1 - step(-100, d_min);
  // gl_FragColor.b = 1 - step(-200, d_min);

  // gl_FragColor.b = (1 - smoothstep(-100, 100 + sin(coord.y / 20) * 10,
  // d_min)); gl_FragColor.r = smoothstep(-100, -90 + sin(coord.y / 200) * 5,
  // d_min); gl_FragColor.g = (1 - smoothstep(-300, -100 + sin(coord.y / 50) *
  // 5, d_min)); gl_FragColor.a = 1.0 - smoothstep(300, 1000, coord.y);
}
