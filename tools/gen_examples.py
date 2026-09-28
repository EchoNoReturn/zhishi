"""Generate 20 golden examples (.zsh source + expected .tex) from the spec §6.

Expected TeX is the contract for zhishi-tex: whitespace-normalized equality.
Run:  python tools/gen_examples.py
"""
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXAMPLES = ROOT / "examples"

CASES = [
    ("01-quadratic",
     "(-b ± sqrt(b^2 - 4*a*c)) / (2*a)",
     r"\frac{-b \pm \sqrt{b^{2} - 4 a c}}{2 a}"),
    ("02-difference-of-squares",
     "(a + b)(a - b) = a^2 - b^2",
     r"(a + b)(a - b) = a^{2} - b^{2}"),
    ("03-nested-fractions",
     "1 / (1 + 1 / (1 + 1/x))",
     r"\frac{1}{1 + \frac{1}{1 + \frac{1}{x}}}"),
    ("04-sum-of-squares",
     "sum_(i=1)^n i^2 = n*(n+1)*(2*n+1)/6",
     r"\sum_{i = 1}^{n} i^{2} = \frac{n(n + 1)(2 n + 1)}{6}"),
    ("05-taylor-exp",
     "exp(x) = sum_(n=0)^oo x^n / n!",
     r"\exp(x) = \sum_{n = 0}^{\infty} \frac{x^{n}}{n!}"),
    ("06-sinc-limit",
     "lim_(x->0) sin(x)/x = 1",
     r"\lim_{x \to 0} \frac{\sin(x)}{x} = 1"),
    ("07-factorial-prod",
     "prod_(k=1)^n k = n!",
     r"\prod_{k = 1}^{n} k = n!"),
    ("08-gaussian-integral",
     "int_0^oo exp(-x^2) dx = sqrt(pi)/2",
     r"\int_{0}^{\infty} \exp(-x^{2}) \, d x = \frac{\sqrt{\pi}}{2}"),
    ("09-newton-leibniz",
     "int_a^b f'(x) dx = f(b) - f(a)",
     r"\int_{a}^{b} f'(x) \, d x = f(b) - f(a)"),
    ("10-wave-equation",
     "partial^2 u/partial t^2 = c^2 partial^2 u/partial x^2",
     r"\frac{\partial^{2} u}{\partial t^{2}} = c^{2} \frac{\partial^{2} u}{\partial x^{2}}"),
    ("11-cases",
     "f(x) = cases{ x^2, x > 0; 0, x <= 0; -x, otherwise }",
     r"f(x) = \begin{cases} x^{2} & x > 0 \\ 0 & x \le 0 \\ -x & \text{otherwise} \end{cases}"),
    ("12-matrix",
     "matrix(1, 2; 3, 4)",
     r"\begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix}"),
    ("13-det",
     "vmatrix(a, b; c, d) = a*d - b*c",
     r"\begin{vmatrix} a & b \\ c & d \end{vmatrix} = a d - b c"),
    ("14-align",
     "align{ 2*x + y &= 5; x - y &= 1 }",
     r"\begin{aligned} 2 x + y &= 5 \\ x - y &= 1 \end{aligned}"),
    ("15-binomial",
     "(x + y)^n = sum_(k=0)^n binom(n, k) x^(n-k) y^k",
     r"(x + y)^{n} = \sum_{k = 0}^{n} \binom{n}{k} x^{n - k} y^{k}"),
    ("16-sample-mean",
     "bar(x) = (1/n) sum_(i=1)^n x_i",
     r"\bar{x} = \frac{1}{n} \sum_{i = 1}^{n} x_{i}"),
    ("17-cauchy-schwarz",
     "|angle(u, v)| <= ||u|| * ||v||",
     r"|\langle u, v \rangle| \le \|u\| \cdot \|v\|"),
    ("18-dot-product",
     "vec(a) · vec(b) = ||a|| ||b|| cos(theta)",
     r"\vec{a} \cdot \vec{b} = \|a\| \|b\| \cos(\theta)"),
    ("19-euler",
     "exp('i * pi) + 1 = 0",
     r"\exp(i \pi) + 1 = 0"),
    ("20-conditional-prob",
     "P(mid(A, B)) = P(A and B) / P(B)",
     r"P(A \mid B) = \frac{P(A \cap B)}{P(B)}"),
]


def main() -> None:
    EXAMPLES.mkdir(parents=True, exist_ok=True)
    for name, src, tex in CASES:
        (EXAMPLES / f"{name}.zsh").write_text(src + "\n", encoding="utf-8")
        (EXAMPLES / f"{name}.tex").write_text(tex + "\n", encoding="utf-8")
    print(f"wrote {len(CASES)} golden cases to {EXAMPLES}")


if __name__ == "__main__":
    main()
