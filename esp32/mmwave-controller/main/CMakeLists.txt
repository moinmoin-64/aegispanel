idf_component_register(
    SRCS "main.cpp" "ld2410.cpp" "power_mgmt.cpp"
    INCLUDE_DIRS "."
    REQUIRES driver esp_timer nvs_flash esp_pm
)
