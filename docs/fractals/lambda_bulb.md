[← Kosalos](kosalos.md) · [All fractals](../FRACTALS.md#bulbs) · [Sine/cosine bulb →](sincos_bulb.md)

# Lambdabulb (`lambda_bulb`)

*Bulbs*

![Lambdabulb](../images/fractals/lambda_bulb.jpg)

The logistic map's Julia set in 3D: a twisted, cratered lump with three-fold symmetry about its axis, its rims seamed with chains of tiny buds. Unlike the bulbs around it, it is a Julia set, and it multiplies by its constant instead of adding one.

**Formula.** Start from $`z = p`$ and repeat:

```math
z \leftarrow c \otimes \left(z - z^{\circ n}\right)
```

where $`z^{\circ n} = r^n(\cos n\theta\cos n\varphi,\; \cos n\theta\sin n\varphi,\; \sin n\theta)`$ with the latitude $`\theta = \arcsin(z/r)`$, and $`\otimes`$ is Nylander's triplex product $`a \otimes b = \left(k(a_x b_x - a_y b_y),\; k(a_x b_y + a_y b_x),\; \rho_b a_z + \rho_a b_z\right)`$ with $`\rho = \sqrt{x^2+y^2}`$ and $`k = 1 - a_z b_z/(\rho_a\rho_b)`$. Distance estimate: $`d = \tfrac12 r\ln r/\partial z`$ with $`\partial z \leftarrow |c|\,(n r^{n-1} + 1)\,\partial z`$; the orbit trap is the nearest the orbit comes to the axis. Its example is tipped 50° so its crater faces the camera.

**Origin.** Ron Barnett ("fractalrebel") posted the [LambdaBulb](https://www.fractalforums.com/mandelbulb-renderings/lambdabulb/) on Fractal Forums on November 23, 2009: a 4th-power lambdabulb with seed $`1.0625 + 0.2375i`$. Paul Nylander rendered it two days later and published the triplex power and product it uses on his [Hypercomplex Fractals](http://bugman123.com/Hypercomplex/) page, crediting Barnett. Graeme McLaren added it to Mandelbulber 2 as `mandelbulb_lambda.cl` in October 2019. Ported from [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2).

**Parameters.** `power` (4): the exponent $`n`$; 4 is Barnett's, higher powers give more lobes (2 to 12). `c` (1.0625, 0.2375, 0): the constant multiplied in each step, as a triplex number ($`x`$ and $`y`$ across its axis, $`z`$ along it). Its length sets how fat the set is (much under 1 swells it past its bounding box), its angle the twist, and $`z`$ tips it out of the plane. `iterations` (12): more give finer detail.

---

[← Kosalos](kosalos.md) · [All fractals](../FRACTALS.md#bulbs) · [Sine/cosine bulb →](sincos_bulb.md)
