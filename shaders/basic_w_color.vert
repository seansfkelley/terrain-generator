#version 150
uniform mat4 mvp;
in vec3 in_Position;
out vec3 fragmentColor;
void main() {
    gl_Position = mvp * vec4(in_Position, 1.0);
    fragmentColor = in_Position;
}
