// Ramp traffic against the ALB to trigger Auto Scaling.
// Usage: k6 run -e BASE_URL=http://<alb-dns-name> loadtest/k6.js
import http from "k6/http";
import { check, sleep } from "k6";

const BASE = __ENV.BASE_URL || "http://localhost:8080";

export const options = {
  stages: [
    { duration: "1m", target: 20 },   // warm up
    { duration: "3m", target: 100 },  // push CPU above the scaling target
    { duration: "5m", target: 100 },  // hold so new instances launch and take traffic
    { duration: "2m", target: 0 },    // ramp down -> scale in
  ],
  thresholds: { http_req_failed: ["rate<0.05"] },
};

export default function () {
  // Mostly cheap reads, with some CPU-heavy requests to actually move CPU utilisation.
  const r = Math.random();
  const res = r < 0.6 ? http.get(`${BASE}/api/books`)
            : r < 0.9 ? http.get(`${BASE}/api/stress?ms=300`)
            : http.get(`${BASE}/api/orders`);
  check(res, { "status 200": (x) => x.status === 200 });
  sleep(0.2);
}
