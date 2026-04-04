from fastapi import FastAPI

app = FastAPI()

@app.get("/orders")
def list_orders():
    return []

@app.post("/orders")
def create_order():
    return {}

@app.get("/orders/{order_id}")
def get_order(order_id: str):
    return {"id": order_id}

# Database connection
db_url = "postgres://localhost:5432/orders"
