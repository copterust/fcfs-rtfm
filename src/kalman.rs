type Float = f32;

pub struct AngularKalman {
    q_a: Float,
    q_b: Float,
    r: Float,
    angle: Float,
    bias: Float,
    rate: Float,
    p: [[Float; 2]; 2],
    k: [Float; 2],
    y: Float,
    s: Float,
}

impl AngularKalman {
    pub fn step(&mut self, angle: Float, rate: Float, dt: Float) -> Float {
        self.rate = rate - self.bias;
        self.angle += self.rate * dt;
        self.p[0][0] += dt * (dt * self.p[1][1] - self.p[0][1] - self.p[1][0] + self.q_a);
        self.p[0][1] -= dt * self.p[1][1];
        self.p[1][0] -= dt * self.p[1][1];
        self.p[1][1] += self.q_b * dt;

        self.s = self.p[0][0] + self.r;
        self.k[0] = self.p[0][0] / self.s;
        self.k[1] = self.p[1][0] / self.s;

        self.y = angle - self.angle;
        self.angle += self.k[0] * self.y;
        self.bias += self.k[1] * self.y;

        self.p[0][0] -= self.k[0] * self.p[0][0];
        self.p[0][1] -= self.k[0] * self.p[0][1];
        self.p[1][0] -= self.k[1] * self.p[0][0];
        self.p[1][1] -= self.k[1] * self.p[0][1];

        return self.angle;
    }
}