import sys
from ddgs import DDGS

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python search.py <query>")
        sys.exit(1)
        
    query = sys.argv[1]
    try:
        results = DDGS().text(query, max_results=3)
        formatted = "\n\n".join([f"Title: {r['title']}\nBody: {r['body']}\nURL: {r['href']}" for r in results])
        print(formatted)
    except Exception as e:
        print(f"Search failed: {e}")
