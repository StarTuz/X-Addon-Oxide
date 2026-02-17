import urllib.request
import urllib.parse
import json

LAT = 51.5703
LON = 0.6933
WIKIDATA_SPARQL_URL = "https://query.wikidata.org/sparql"

def run_query():
    point = f"Point({LON} {LAT})"
    
    # Combined Logic:
    # 1. Primary Query: Direct Coords
    # 2. Secondary Query: Tenants (Clubs)
    # 3. Deduplication: Remove Primary if it matches Secondary's Venue
    # 4. Filter: Remove "Roller Coaster" (Q204832) if "Amusement Park" (Q194195) is present? 
    #    Actually simple filter: Filter OUT Q204832 (Roller Coaster) entirely?
    #    User said "we have adventure island, we don't need rage". Rage is a roller coaster.
    
    query = f"""
PREFIX geo: <http://www.opengis.net/ont/geosparql#>
PREFIX wikibase: <http://wikiba.se/ontology#>
PREFIX wd: <http://www.wikidata.org/entity/>
PREFIX wdt: <http://www.wikidata.org/prop/direct/>
PREFIX schema: <http://schema.org/>

SELECT ?place ?placeLabel ?location ?sitelinks ?type ?typeLabel ?venue WHERE {{
  {{
    # PRIMARY QUERY
    SERVICE wikibase:around {{
      ?place wdt:P625 ?location .
      bd:serviceParam wikibase:center "{point}"^^geo:wktLiteral .
      bd:serviceParam wikibase:radius "20" .
    }}
    ?place wdt:P31/wdt:P279* ?type .
    # Added Q204832 (Roller Coaster) to see if it appears, so we know what to block
    VALUES ?type {{ wd:Q483110 wd:Q33506 wd:Q570116 wd:Q2319498 wd:Q370597 wd:Q476028 wd:Q194195 wd:Q204832 }}
  }} UNION {{
    # SECONDARY QUERY (Tenants)
    SERVICE wikibase:around {{
      ?venue wdt:P625 ?location .
      bd:serviceParam wikibase:center "{point}"^^geo:wktLiteral .
      bd:serviceParam wikibase:radius "20" .
    }}
    ?place wdt:P115 ?venue .
    ?place wdt:P31/wdt:P279* ?type .
    VALUES ?type {{ wd:Q476028 }}
  }}

  ?place wikibase:sitelinks ?sitelinks .
  FILTER(?sitelinks > 3)
  
  ?article schema:about ?place .
  ?article schema:inLanguage "en" .
  ?article schema:isPartOf <https://en.wikipedia.org/> .
  
  SERVICE wikibase:label {{ bd:serviceParam wikibase:language "en". }}
  OPTIONAL {{ ?place wdt:P31 ?directType . ?directType rdfs:label ?typeLabel . FILTER(LANG(?typeLabel) = "en") }}
}} ORDER BY DESC(?sitelinks)
LIMIT 50
"""

    print(f"Querying Wikidata near {LAT}, {LON}...")
    
    params = {"query": query, "format": "json"}
    url = f"{WIKIDATA_SPARQL_URL}?{urllib.parse.urlencode(params)}"
    
    req = urllib.request.Request(url)
    req.add_header("User-Agent", "X-Addon-Oxide-Debug/1.0")
    req.add_header("Accept", "application/sparql-results+json")
    
    try:
        with urllib.request.urlopen(req) as response:
            data = json.loads(response.read().decode())
            
        bindings = data["results"]["bindings"]
        
        # Simulating Logic
        ignored_venue_uris = set()
        football_clubs = []
        raw_primary = []
        
        # Pass 1: Identify Tenants and their Venues
        for row in bindings:
            uri = row.get("place", {}).get("value")
            venue = row.get("venue", {}).get("value")
            type_uri = row.get("type", {}).get("value")
            
            if venue and "Q476028" in type_uri: # Is Club via Tenant Query
                ignored_venue_uris.add(venue)
                football_clubs.append(row)
            else:
                raw_primary.append(row)

        print(f"Ignored Venue URIs: {ignored_venue_uris}")
        
        print(f"\n{'Label':<40} | {'Sitelinks':<10} | {'Type':<30} | {'Action'}")
        print("-" * 100)
        
        # process list
        final_list = []
        # Merge back
        all_rows = football_clubs + raw_primary
        # Sort by sitelinks (simulated)
        all_rows.sort(key=lambda x: int(x.get("sitelinks", {}).get("value", 0)), reverse=True)
        
        seen = set()
        
        for row in all_rows:
            label = row.get("placeLabel", {}).get("value", "Unknown")
            uri = row.get("place", {}).get("value")
            type_label = row.get("typeLabel", {}).get("value", "Unknown")
            sitelinks = int(row.get("sitelinks", {}).get("value", 0))
            
            action = "KEEP"
            
            # Dedupe
            if uri in seen:
                continue
            seen.add(uri)
            
            # VENUE CHECK
            if uri in ignored_venue_uris:
                action = "DROP (Venue of Club)"
                
            # ROLLER COASTER CHECK (Rage is Q204832)
            # Actually need to check if type is Roller Coaster.
            # In my query I added Q204832 to VALUES to see if it catches Rage.
            # If so, we can just NOT include Q204832 in the final Rust code.
            
            print(f"{label:<40} | {sitelinks:<10} | {type_label:<30} | {action}")

    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    run_query()
