-- Generates realistic nested JSONB documents mimicking CQRS architecture
-- Scale: 100 network configurations × 50 DNS servers each

CREATE OR REPLACE FUNCTION generate_cqrs_test_data()
RETURNS void AS $$
BEGIN
    -- Drop existing test tables
    DROP TABLE IF EXISTS bench_dns_servers CASCADE;
    DROP TABLE IF EXISTS bench_network_configs CASCADE;
    DROP TABLE IF EXISTS bench_allocations CASCADE;
    DROP TABLE IF EXISTS bench_nc_dns_mapping CASCADE;

    -- Base table: DNS servers (500 records)
    CREATE TABLE bench_dns_servers (
        id INTEGER PRIMARY KEY,
        ip TEXT NOT NULL,
        port INTEGER NOT NULL,
        status TEXT NOT NULL
    );

    INSERT INTO bench_dns_servers
    SELECT
        i,
        '8.8.' || (i % 256) || '.' || ((i / 256) % 256),
        53 + (i % 10),
        CASE WHEN i % 5 = 0 THEN 'inactive' ELSE 'active' END
    FROM generate_series(1, 500) i;

    -- Materialized view: v_dns_server (leaf view)
    CREATE MATERIALIZED VIEW v_dns_server AS
    SELECT
        id,
        jsonb_build_object(
            'id', id,
            'ip', ip,
            'port', port,
            'status', status
        ) AS data
    FROM bench_dns_servers;

    CREATE INDEX idx_v_dns_server_id ON v_dns_server(id);

    -- Base table: Network configurations (100 records)
    CREATE TABLE bench_network_configs (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        gateway_ip TEXT NOT NULL,
        subnet_mask TEXT NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT now()
    );

    INSERT INTO bench_network_configs
    SELECT
        i,
        'Network Config ' || i,
        '192.168.' || (i % 256) || '.1',
        '255.255.255.0',
        now() - (i || ' days')::interval
    FROM generate_series(1, 100) i;

    -- Mapping table: Network Config ↔ DNS Servers (50 DNS servers per config)
    CREATE TABLE bench_nc_dns_mapping (
        network_configuration_id INTEGER REFERENCES bench_network_configs(id),
        dns_server_id INTEGER REFERENCES bench_dns_servers(id),
        priority INTEGER NOT NULL,
        PRIMARY KEY (network_configuration_id, dns_server_id)
    );

    INSERT INTO bench_nc_dns_mapping
    SELECT
        ((i - 1) / 50) + 1 AS network_configuration_id,
        ((i - 1) % 50) + 1 AS dns_server_id,
        (i % 50) + 1 AS priority
    FROM generate_series(1, 5000) i;

    -- Materialized view: tv_network_configuration (intermediate composition)
    CREATE MATERIALIZED VIEW tv_network_configuration AS
    SELECT
        nc.id,
        jsonb_build_object(
            'id', nc.id,
            'name', nc.name,
            'gateway_ip', nc.gateway_ip,
            'subnet_mask', nc.subnet_mask,
            'dns_servers', COALESCE((
                SELECT jsonb_agg(v.data ORDER BY m.priority)
                FROM bench_nc_dns_mapping m
                JOIN v_dns_server v ON v.id = m.dns_server_id
                WHERE m.network_configuration_id = nc.id
            ), '[]'::jsonb),
            'created_at', nc.created_at
        ) AS data
    FROM bench_network_configs nc;

    CREATE INDEX idx_tv_network_configuration_id ON tv_network_configuration(id);

    -- Base table: Allocations (500 records, 5 per network config)
    CREATE TABLE bench_allocations (
        id INTEGER PRIMARY KEY,
        name TEXT NOT NULL,
        network_configuration_id INTEGER REFERENCES bench_network_configs(id),
        machine_name TEXT NOT NULL,
        storage_size_gb INTEGER NOT NULL
    );

    INSERT INTO bench_allocations
    SELECT
        i,
        'Allocation ' || i,
        ((i - 1) / 5) + 1 AS network_configuration_id,
        'Machine-' || i,
        (i % 10 + 1) * 100
    FROM generate_series(1, 500) i;

    -- Materialized view: tv_allocation (top-level composition)
    CREATE MATERIALIZED VIEW tv_allocation AS
    SELECT
        a.id,
        jsonb_build_object(
            'id', a.id,
            'name', a.name,
            'network_configuration', nc.data,  -- ← Nested 10KB+ document
            'machine', jsonb_build_object(
                'name', a.machine_name,
                'status', 'active'
            ),
            'storage', jsonb_build_object(
                'size_gb', a.storage_size_gb,
                'type', 'SSD'
            )
        ) AS data
    FROM bench_allocations a
    LEFT JOIN tv_network_configuration nc ON nc.id = a.network_configuration_id;

    CREATE INDEX idx_tv_allocation_id ON tv_allocation(id);
    CREATE INDEX idx_tv_allocation_nc_id ON tv_allocation((data->'network_configuration'->>'id'));

    RAISE NOTICE 'Test data generated successfully:';
    RAISE NOTICE '  - 500 DNS servers';
    RAISE NOTICE '  - 100 network configurations (50 DNS servers each)';
    RAISE NOTICE '  - 500 allocations (5 per network config)';
    RAISE NOTICE '  - Total JSONB size: ~50MB';
END;
$$ LANGUAGE plpgsql;

-- Execute generator
SELECT generate_cqrs_test_data();

-- Verify data sizes
SELECT
    'v_dns_server' AS view,
    COUNT(*) AS rows,
    pg_size_pretty(pg_total_relation_size('v_dns_server')) AS size
FROM v_dns_server
UNION ALL
SELECT
    'tv_network_configuration',
    COUNT(*),
    pg_size_pretty(pg_total_relation_size('tv_network_configuration'))
FROM tv_network_configuration
UNION ALL
SELECT
    'tv_allocation',
    COUNT(*),
    pg_size_pretty(pg_total_relation_size('tv_allocation'))
FROM tv_allocation;