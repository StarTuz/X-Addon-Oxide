import urllib.request
import urllib.parse
import json
import sys

# EGMC (Southend) Approximate Coordinates
LAT = 51.5703
LON = 0.6933

WIKIDATA_SPARQL_URL = "https://query.wikidata.org/sparql"

def run_query():
    point = f"Point({LON} {LAT})"
    
    # Complex query using UNION to find items via their location/venue
    query = f"""
PREFIX geo: <http://www.opengis.net/ont/geosparql#>
PREFIX wikibase: <http://wikiba.se/ontology#>
PREFIX wd: <http://www.wikidata.org/entity/>
PREFIX wdt: <http://www.wikidata.org/prop/direct/>
PREFIX schema: <http://schema.org/>

SELECT DISTINCT ?placeLabel ?location ?article ?sitelinks ?typeLabel WHERE {{
  SERVICE wikibase:around {{
    ?locItem wdt:P625 ?location .
    bd:serviceParam wikibase:center "{point}"^^geo:wktLiteral .
    bd:serviceParam wikibase:radius "20" .
  }}
  
  {{
    BIND(?locItem AS ?place)
  }} UNION {{
    ?place wdt:P115 ?locItem . # Club plays at Venue
  }} UNION {{
    ?place wdt:P159 ?locItem . # Org HQ is at Location
  }}
  
  ?place wdt:P31/wdt:P279* ?type .
  VALUES ?type {{ wd:Q483110 wd:Q33506 wd:Q570116 wd:Q2319498 wd:Q370597 wd:Q476028 wd:Q194195 }}
  
  ?place wikibase:sitelinks ?sitelinks .
  FILTER(?sitelinks > 3)
  
  ?article schema:about ?place .
  ?article schema:inLanguage "en" .
  ?article schema:isPartOf <https://en.wikipedia.org/> .
  
  SERVICE wikibase:label {{ bd:serviceParam wikibase:language "en". }}
}} ORDER BY DESC(?sitelinks)
LIMIT 40
"""

    print(f"Querying Wikidata with UNION for POIs near {LAT}, {LON}...")
    
    params = {
        "query": query,
        "format": "json"
    }
    url = f"{WIKIDATA_SPARQL_URL}?{urllib.parse.urlencode(params)}"
    
    req = urllib.request.Request(url)
    req.add_header("User-Agent", "X-Addon-Oxide-Debug/1.0")
    req.add_header("Accept", "application/sparql-results+json")
    
    try:
        with urllib.request.urlopen(req) as response:
            data = json.loads(response.read().decode())
            
        bindings = data["results"]["bindings"]
        print(f"Found {len(bindings)} results:\n")
        print(f"{'Label':<40} | {'Sitelinks':<10} | {'Type':<40}")
        print("-" * 95)
        
        for row in bindings:
            label = row.get("placeLabel", {}).get("value", "Unknown")
            sitelinks = int(row.get("sitelinks", {}).get("value", 0))
            type_label = row.get("typeLabel", {}).get("value", "Unknown")
            type_uri = row.get("type", {}).get("value", "")
            
            print(f"{label:<40} | {sitelinks:<10} | {type_label:<40}")

    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    run_query()
