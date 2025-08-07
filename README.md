# Overview
This project aims to map raiders who frequently complete trials in The Elder Scrolls Online. 

This is accomplished by iterating through public logs on esologs.com and gathering which players appear in them, and using that data to create a visualisation.

# Process

## Collection
All of my data collection was done using the [esologs.com v2 API](https://www.esologs.com/v2-api-docs/eso/). 

I collected over 1.2m public report codes by iterating through every public user id. They are indexed sequentially which made it trivial to set up a script to do this.

Then I set about collecting the player information from each of those logs. I tested various methods according to the API documentation in order to reduce the token cost as much as possible. There is supposed to be a data object that has all ranked players from a log, but I found that it returned nil results almost all of the time even when the web interface showed reasonable data should be returned so I ignored it. Instead, I dumped all objects of type "player" from the actor table.

Despite my best efforts, this script was still quite slow. My 1.2m public reports were being processed at 3600 logs per hour maximum (due to rate limiting). This meant that to get all of the data it would take 14 days! Unfortunately after ~6 days of leaving the script idly running my API key was deactivated for abuse because scraping their database using the API is prohibited. I still managed to get 690k report records in that time and that is good enough for this project.

## Processing
After collating my data, I chose parameters to reduce the amount of data I would be visualising. It would be impractical to view all 248k unique players and their edges, and mostly useless since many only appear a couple of times.

Instead, I did the following:
- If a log contains 30 or more unique players, remove it (A great many logs are filled with hundreds of unique players. Very alarming)
- If a player does not appear 200 or more times, remove them
- Increase player weights by the duration of the log, up to two hours per log.
- Remove edges that are less than one day in weight
- Keep only the highest weighted 30 edges per player
- Make the weight logarithmic base 10.

After following these steps I had just 5,935 unique players, and 52,951 edges between them. Very manageable!

I created a [Graph Exchange XML Format](https://gexf.net/) (.gexf) file using this data that described the nodes and their edges.

## Visualisation
The visualisation is made using [Gephi](https://gephi.org/), a free open source visualization and exploration software for all kinds of graphs and networks. 

I imported the gexf file I made beforehand directly into Gephi. The graph starts out as a square though, so a layout function has to be iteratively run to move nodes to the appropriate location. The specific function I found best is ForceAtlas2. I am by no means a graphing expert, this is just what I found after trying for a number of hours.

The specific settings I used are:
- Gravity: 4.0
- [x] LinLog Mode
- [x] Prevent Overlap
- Edge weight influence: 1.0

Everything else was left as default or unchecked.

The graph created in Gephi was exported to a json file (done via [JSONExporter plugin](https://github.com/oxfordinternetinstitute/gephi-plugins/tree/jsonexporter-plugin)), and rendered using two html canvas elements layered on top of one another: one for the edges, one for the nodes. This was done so that edges could slowly render in using the comparatively performant requestAnimationFrame() Javascript function instead of drawing tens of thousands of edges at once every frame.

Building the visualisation off an exported Gephi file allows it to be easily changed in future, should additional data become available or necessary.

The website itself is built using the [Yew](https://yew.rs/) framework. 