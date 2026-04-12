from fastapi import FastAPI

app = FastAPI()

# mongodb://localhost:27017/catalog
DB = "mongodb://localhost:27017/catalog"

@app.get("/items")
def list_items():
    return []

@app.post("/items")
def create_item():
    return {"ok": True}
