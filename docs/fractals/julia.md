[← Mandelbox](mandelbox.md) · [All fractals](../FRACTALS.md#the-classics) · [Sierpinski Tetrahedron →](sierpinski.md)

# Quaternion Julia (`julia`)

*The classics*

![Quaternion Julia](../images/fractals/julia.jpg)

A Julia set of $`z \mapsto z^2 + c`$ over the quaternions, a 4D object, sliced in 3D where the fourth coordinate is 0. It is smooth and organic, with swirling shells and whorls, and a small change to $`c`$ reshapes it completely.

**Formula.** Start from the quaternion $`z = (p_x, p_y, p_z, 0)`$, whose first component is the real part, and $`|z'|^2 = 1`$. While $`|z|^2 \lt  16`$, for at most `iterations` steps, repeat:

```math
|z'|^2 \leftarrow 4\,|z|^2\,|z'|^2, \qquad z \leftarrow z^2 + c
```

where for $`z = (a, \mathbf{v})`$ with real part $`a`$ and vector part $`\mathbf{v}`$, $`z^2 = (a^2 - |\mathbf{v}|^2,\ 2a\,\mathbf{v})`$. Only the derivative's length is tracked, with the scalar rule $`|z'| \leftarrow 2\,|z|\,|z'|`$, as for complex numbers. Distance estimate:

```math
d = \tfrac14 \sqrt{|z|^2 / |z'|^2}\ \ln |z|^2 = \frac{|z| \ln |z|}{2\,|z'|}
```

Orbit trap: the smallest $`|z|`$ along the orbit, the starting point included.

**Origin.** Alan Norton first generated and displayed quaternion Julia sets as 3D slices in "Generation and display of geometric fractals in 3-D" ([SIGGRAPH 1982](https://history.siggraph.org/?p=106343)). John Hart, Daniel Sandin and Louis Kauffman ray-traced them with a distance estimate in "Ray tracing deterministic 3-D fractals" ([SIGGRAPH 1989](https://history.siggraph.org/?p=119380)). ray's form, which tracks only $`|z'|^2`$, is Inigo Quilez's: see his [3D Julia sets](https://iquilezles.org/articles/juliasets3d/) article and the Shadertoy [Quaternion Julia](https://www.shadertoy.com/view/MsfGRr) (2013, a port of his 2007 demo Kindernoiser). ray bails out at $`|z|^2 = 16`$ where iq uses 4.

**Parameters.** `c` ((-0.2, 0.6, 0.2, 0.2)): the constant quaternion (x, y, z, w), x being the real part. Small changes reshape the set completely, and constants too far from the origin break it into scattered pieces. `iterations` (10): escape-time steps, 1 to 32; more give finer detail. Iterations are not added close up.

---

[← Mandelbox](mandelbox.md) · [All fractals](../FRACTALS.md#the-classics) · [Sierpinski Tetrahedron →](sierpinski.md)
