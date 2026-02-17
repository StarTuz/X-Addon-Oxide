import urllib.request
import urllib.parse
import json

WIKIDATA_SPARQL_URL = "https://query.wikidata.org/sparql"

def check_item():
    query = """
PREFIX wd: <http://www.wikidata.org/entity/>
PREFIX wdt: <http://www.wikidata.org/prop/direct/>
PREFIX wikibase: <http://wikiba.se/ontology#>
PREFIX schema: <http://schema.org/>

SELECT ?label ?sitelinks ?coord ?type ?typeLabel WHERE {
  BIND(wd:Q7570498 AS ?item)
  ?item rdfs:label ?label .
  FILTER(LANG(?label) = "en")
  
  OPTIONAL { ?item wdt:P625 ?coord . }
  OPTIONAL { ?item wikibase:sitelinks ?sitelinks . }
  OPTIONAL { 
    ?item wdt:P31/wdt:P279* ?type .
    ?type rdfs:label ?typeLabel .
    FILTER(LANG(?typeLabel) = "en")
  }
}
"""
    print("Checking Southend Pier (Q7570498)...")
    
    params = {"query": query, "format": "json"}
    url = f"{WIKIDATA_SPARQL_URL}?{urllib.parse.urlencode(params)}"
    
    req = urllib.request.Request(url)
    req.add_header("User-Agent", "X-Addon-Oxide-Debug/1.0")
    
    try:
        with urllib.request.urlopen(req) as response:
            data = json.loads(response.read().decode())
            
        for row in data["results"]["bindings"]:
            print(json.dumps(row, indent=2))
            
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    check_item()
