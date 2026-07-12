# Climate

Climate is the long-term pattern of weather in a region. It determines agriculture, settlement patterns, disease ecology, and many other simulation outcomes.

## Climate Representation

Climate is represented as a field of parameters:

```text
ClimateState:
    temperature: TemperatureField
    precipitation: PrecipitationField
    humidity: HumidityField
    wind: WindField
    pressure: PressureField
    insolation: InsolationField
    seasonality: SeasonalityParameters
```

### Temperature Field

```text
TemperatureField:
    mean_annual: ScalarField
    diurnal_range: ScalarField
    seasonal_range: ScalarField
    extreme_minimum: ScalarField
    extreme_maximum: ScalarField
```

### Precipitation Field

```text
PrecipitationField:
    mean_annual: ScalarField
    seasonal_distribution: [float]  -- monthly proportions
    variability: ScalarField
```

## Climate Drivers

Climate is driven by:

- **Latitude**: solar angle, day length variation
- **Elevation**: temperature lapse rate
- **Aspect**: sun exposure
- **Distance to water**: maritime influence
- **Topography**: rain shadows, valley effects
- **Atmospheric circulation**: prevailing winds, pressure systems
- **Ocean currents**: heat transport

## Climate and Other Domains

Climate interacts with:

- **Terrain**: temperature varies with elevation
- **Hydrology**: precipitation drives water availability
- **Ecology**: climate determines biome boundaries
- **Agriculture**: growing season length, crop suitability
- **Health**: temperature and humidity affect disease transmission
- **Settlement**: extreme climates discourage habitation
- **Mana**: atmospheric patterns may interact with mana fields

## Seasonality

Climate includes seasonal variation:

- **Growing season**: frost-free period
- **Wet/dry seasons**: precipitation timing
- **Temperature extremes**: seasonal minima and maxima

Seasonality affects:
- agricultural calendars
- migration patterns
- disease cycles
- construction schedules
- military campaigns

## Climate Change

The simulation may include long-term climate variation:

- **Gradual change**: slow shifts in temperature or precipitation
- **Extreme events**: droughts, floods, storms
- **Catastrophic events**: volcanic winters, meteor impacts

Climate change creates historical pressure:
- crop failures
- migration
- resource conflicts
- technological adaptation

## Determinism

Climate generation must be deterministic given:

- world_seed
- orbital parameters
- atmospheric parameters
- ocean parameters

## Performance

Climate data is mostly static at short timescales. Strategies:

- Precompute climate fields at generation time
- Update only during climate change events
- Use GPU for climate field operations
- Compress climate data for storage

## Related Documents

- `geography-philosophy.md` — geographic causality
- `terrain.md` — elevation effects on climate
- `hydrology.md` — precipitation and evaporation
- `ecology.md` — biome determination
- `world-generation-provenance.md` — provenance tracking

## TODO Categories

- `CLIMATE` — climate systems
- `WORLD` — general world systems
