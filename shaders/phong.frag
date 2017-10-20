#version 410

uniform vec3 u_LightPosition_WorldSpace;
uniform vec3 u_LightColor;
uniform float u_LightPower;

in vec3 out_ColorAmbient;
in vec3 out_ColorDiffuse;
in vec3 out_ColorSpecular;
in float out_SpecularExponent;
in vec3 out_VertexPosition_WorldSpace;
in vec3 out_EyeDirection_CameraSpace;
in vec3 out_LightDirection_CameraSpace;
in vec3 out_VertexNormal_CameraSpace;

out vec3 color;

void main() {
    vec3 normal_VertexNormal = normalize(out_VertexNormal_CameraSpace);
    vec3 normal_LightDirection = normalize(out_LightDirection_CameraSpace);
    vec3 normal_EyeDirection = normalize(out_EyeDirection_CameraSpace);

    float cosTheta = clamp(dot(normal_VertexNormal, normal_LightDirection), 0, 1);

    vec3 normal_Reflect_EyeDirection = reflect(-normal_LightDirection, normal_VertexNormal);
    float cosAlpha = clamp(dot(normal_EyeDirection, normal_Reflect_EyeDirection), 0, 1);

    float distance = length(u_LightPosition_WorldSpace - out_VertexPosition_WorldSpace);
    color =
      out_ColorAmbient +
      out_ColorDiffuse * u_LightColor * u_LightPower * cosTheta / (distance * distance) +
      out_ColorSpecular * u_LightColor * u_LightPower * pow(cosAlpha, out_SpecularExponent) / (distance * distance);
}
