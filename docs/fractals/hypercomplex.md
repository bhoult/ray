[← Tetrabrot](tetrabrot.md) · [All fractals](../FRACTALS.md#algebras) · [Aexion →](aexion.md)

# Hypercomplex Mandelbrot (`hypercomplex`)

*Algebras*

![Hypercomplex Mandelbrot](../images/fractals/hypercomplex.jpg)

Clyde Davenport's commutative quaternions, iterated as $`z^2 + c`$ and sliced through $`1`$, $`i`$ and $`k`$. They are the bicomplex numbers again, so the set is two Mandelbrot sets crossed, but this slice draws them out along the real axis instead of across it: a stepped arrowhead of stacked slabs with pyramid-like bulbs. Turning the slice toward $`j`$ morphs it into the Tetrabrot.

**Formula.** Davenport's units have $`i^2 = j^2 = -1`$, $`k^2 = +1`$, $`ij = k`$, $`ik = -j`$ and $`jk = -i`$, so

```math
(a + b\,i + c\,j + d\,k)^2 = (a^2 - b^2 - c^2 + d^2) + 2(ab - cd)\,i + 2(ac - bd)\,j + 2(ad + bc)\,k.
```

With the slice angle $`\theta`$, the point is $`c = x + y\,i + u\,j + v\,k`$ with $`x = p_x - 0.75`$, $`y = p_y`$, $`u = p_z \sin\theta + w\cos\theta`$ and $`v = p_z\cos\theta - w\sin\theta`$. Its idempotent components are $`c_1 = (x + v) + (y - u)\,i`$ and $`c_2 = (x - v) + (y + u)\,i`$, each iterated as a complex number $`u_k \leftarrow u_k^2 + c_k`$, with the Tetrabrot's distance estimate $`d = \sqrt{(d_1^2 + d_2^2)/2}`$.

**Origin.** Clyde M. Davenport, *A Hypercomplex Calculus with Applications to Relativity* (self-published, ISBN 0-9623837-0-8), the basis of Fractint's hypercomplex fractals (Tim Wegner's [`hcmplx.c`](https://github.com/jhol/fractint/blob/master/common/hcmplx.c) has the same square). The 1-$`i`$-$`k`$ slice is the bicomplex Mandelbrot set's "Arrowheadbrot" in Brouillette, Parisé and Rochon's [catalog of principal slices](https://arxiv.org/abs/1811.09697) (2019). Mandelbulber 2's ["Hypercomplex"](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/hypercomplex.cl) formula squares with $`k^2 = -1`$ and $`ij = -k`$ instead: a non-associative algebra in which squaring scales $`|z|^2`$ by anything from 0.35 to 1.8 depending on direction, so its escape potential is too rough for a usable distance estimate (the estimate jumps from one escape iteration to the next). Davenport's algebra is used here.

**Parameters.** `angle` (0, from 0 to 90): the slice's third axis, turned from $`k`$ (0: the arrowhead) toward $`j`$ (90: the Tetrabrot), in degrees. In between, the crossed Mandelbrot sets lean and the bulbs become slanted wedges. `w` (0): the slice's fourth coordinate. It slides the two Mandelbrot sets apart, so the shape shrinks and splits into crossed bulbs. `iterations` (16, up to 40): escape-time iterations; more give sharper edges and finer bulbs.

---

[← Tetrabrot](tetrabrot.md) · [All fractals](../FRACTALS.md#algebras) · [Aexion →](aexion.md)
