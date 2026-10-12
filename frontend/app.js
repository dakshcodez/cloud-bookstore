const $ = (id) => document.getElementById(id);
const money = (cents) => `$${(cents / 100).toFixed(2)}`;

function cell(text) {
  const td = document.createElement("td");
  td.textContent = text;
  return td;
}

async function api(path, options) {
  const res = await fetch(path, options);
  if (!res.ok) throw new Error((await res.text()) || res.statusText);
  return res.json();
}

async function loadInfo() {
  try {
    const i = await api("/api/info");
    $("served-by").textContent = `Served by: ${i.instance_id} (${i.availability_zone})`;
  } catch {
    $("served-by").textContent = "Served by: unknown";
  }
}

async function loadBooks() {
  const books = await api("/api/books");
  const tbody = $("books").tBodies[0];
  const select = $("book-select");
  tbody.replaceChildren();
  select.replaceChildren();
  for (const b of books) {
    const tr = document.createElement("tr");
    const btn = document.createElement("button");
    btn.textContent = "Order";
    btn.type = "button";
    btn.onclick = () => { select.value = b.id; $("customer").focus(); };
    const action = document.createElement("td");
    action.append(btn);
    tr.append(cell(b.title), cell(b.author), cell(money(b.price_cents)), action);
    tbody.append(tr);

    const opt = document.createElement("option");
    opt.value = b.id;
    opt.textContent = `${b.title} (${money(b.price_cents)})`;
    select.append(opt);
  }
}

async function loadOrders() {
  const orders = await api("/api/orders");
  const tbody = $("orders").tBodies[0];
  tbody.replaceChildren();
  for (const o of orders) {
    const tr = document.createElement("tr");
    tr.append(cell(o.id), cell(o.customer_name), cell(o.book_title), cell(new Date(o.created_at).toLocaleString()));
    tbody.append(tr);
  }
}

$("order-form").addEventListener("submit", async (e) => {
  e.preventDefault();
  const msg = $("message");
  try {
    const order = await api("/api/orders", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ book_id: Number($("book-select").value), customer_name: $("customer").value }),
    });
    msg.className = "ok";
    msg.textContent = `Order #${order.id} placed for "${order.book_title}".`;
    $("customer").value = "";
    loadOrders();
  } catch (err) {
    msg.className = "error";
    msg.textContent = `Failed: ${err.message}`;
  }
});

loadInfo();
loadBooks().catch((e) => { $("message").textContent = e.message; });
loadOrders().catch(() => {});
