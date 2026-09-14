shader_type spatial;
render_mode diffuse_burley, specular_schlick_ggx, cull_back;

// Dá um visual de contêiner de verdade (chapa corrugada + porta + sujeira)
// a uma caixa lisa, sem precisar de textura ou modelo externo.

uniform vec4 cor_base : source_color = vec4(0.5, 0.3, 0.2, 1.0);
uniform float metalico : hint_range(0.0, 1.0) = 0.25;
uniform float aspereza_base : hint_range(0.0, 1.0) = 0.85;

uniform float freq_corrugado : hint_range(2.0, 60.0) = 20.0;
uniform float prof_corrugado : hint_range(0.0, 1.0) = 0.35;

// Varia por contêiner mesmo usando o mesmo material (economiza recursos)
instance uniform float semente : hint_range(0.0, 1000.0) = 0.0;
instance uniform float variacao_cor : hint_range(-0.25, 0.25) = 0.0;
instance uniform float flash : hint_range(0.0, 1.0) = 0.0;

float hash(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

void fragment() {
	vec2 uv = UV;

	// corrugação vertical (ranhuras de cima a baixo, como a chapa real)
	float ridge = sin(uv.x * freq_corrugado * 6.28318);
	NORMAL = normalize(NORMAL + TANGENT * ridge * prof_corrugado);
	float sombra_ridge = mix(0.82, 1.0, 0.5 + 0.5 * ridge);

	// porta central, com uma fresta vertical no meio
	float porta_x = step(0.26, uv.x) * step(uv.x, 0.74);
	float porta_y = step(0.32, uv.y);
	float porta = porta_x * porta_y;
	float fresta = (1.0 - smoothstep(0.0, 0.01, abs(uv.x - 0.5))) * porta_y;

	// sujeira/ferrugem — varia por contêiner via "semente" de instância
	float ruido = hash(floor(uv * vec2(5.0, 3.0)) + semente);
	float sujeira = smoothstep(0.62, 1.0, ruido);

	vec3 cor = cor_base.rgb * (1.0 + variacao_cor);
	cor *= sombra_ridge;
	cor = mix(cor, cor * 0.6, porta * 0.35);
	cor = mix(cor, vec3(0.04), fresta * 0.7);
	cor = mix(cor, cor * vec3(0.5, 0.42, 0.36), sujeira * 0.35);
	cor = mix(cor, vec3(1.0, 0.8, 0.4), flash);

	ALBEDO = cor;
	METALLIC = metalico;
	ROUGHNESS = clamp(aspereza_base - sujeira * 0.15, 0.05, 1.0);
}
