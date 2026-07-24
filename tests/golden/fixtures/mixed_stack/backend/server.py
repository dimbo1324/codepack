from fastapi import FastAPI

api = FastAPI()


@api.get("/items")
def list_items():
    return []
