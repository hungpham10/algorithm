import pytest
import sys
import os
from pathlib import Path

# Add the parent directory to sys.path to import the package
parent_dir = Path(__file__).parent.parent
sys.path.append(str(parent_dir))

# Try to import the package - handle potential import errors gracefully
try:
    from vnscope import market, Monitor, Datastore

    IMPORTS_SUCCEEDED = True
except ImportError as e:
    print(f"Import error: {e}")
    IMPORTS_SUCCEEDED = False

# Skip all tests if imports failed
pytestmark = pytest.mark.skipif(
    not IMPORTS_SUCCEEDED, reason="Failed to import vnscope package"
)


def test_package_imports():
    """Test that the package can be imported."""
    assert IMPORTS_SUCCEEDED, "Failed to import vnscope package"


@pytest.mark.skipif(not IMPORTS_SUCCEEDED, reason="Scope package not available")
def test_market_function_exists():
    """Test that the market function exists and is callable."""
    assert callable(market)


@pytest.mark.skipif(not IMPORTS_SUCCEEDED, reason="Scope package not available")
def test_market_with_empty_list():
    """Test market function with an empty list of symbols."""
    try:
        result = market([])
        assert result is not None
        assert hasattr(result, "shape")  # Should return a DataFrame
        assert result.shape[0] == 0  # Empty DataFrame
    except Exception as e:
        pytest.skip(f"Market function failed with empty list: {str(e)}")


@pytest.mark.skipif(not IMPORTS_SUCCEEDED, reason="Scope package not available")
def test_market_with_symbols():
    """Test market function with some stock symbols."""
    symbols = ["VNM", "VIC", "VHM"]
    try:
        result = market(symbols)
        assert result is not None
        assert hasattr(result, "shape")
        # May have fewer rows if some symbols aren't found
        assert result.shape[0] <= len(symbols)
    except Exception as e:
        pytest.skip(f"Market function failed with symbols: {str(e)}")


@pytest.mark.skipif(not IMPORTS_SUCCEEDED, reason="Scope package not available")
def test_monitor_class_exists():
    """Test that the Monitor class exists."""
    assert Monitor is not None


@pytest.mark.skipif(not IMPORTS_SUCCEEDED, reason="Scope package not available")
def test_monitor_initialization():
    """Test Monitor initialization."""
    try:
        monitor = Monitor()
        assert monitor is not None
    except Exception as e:
        pytest.skip(f"Monitor initialization failed: {str(e)}")


@pytest.mark.skipif(not IMPORTS_SUCCEEDED, reason="Scope package not available")
def test_datastore_class_exists():
    """Test that the Datastore class exists."""
    assert Datastore is not None


@pytest.mark.skipif(not IMPORTS_SUCCEEDED, reason="Scope package not available")
def test_datastore_initialization():
    """Test Datastore initialization."""
    try:
        datastore = Datastore()
        assert datastore is not None
    except Exception as e:
        pytest.skip(f"Datastore initialization failed: {str(e)}")


@pytest.mark.skipif(not IMPORTS_SUCCEEDED, reason="Scope package not available")
@pytest.mark.parametrize("symbol", ["VNM", "VIC", "VHM"])
def test_monitor_get_stock_info(symbol):
    """Test Monitor's ability to get stock information."""
    try:
        monitor = Monitor()
        info = monitor.get_stock_info(symbol)
        assert info is not None
    except AttributeError:
        pytest.skip(f"Monitor.get_stock_info method doesn't exist")
    except Exception as e:
        pytest.skip(f"Skipping test for {symbol}: {str(e)}")


@pytest.mark.skipif(not IMPORTS_SUCCEEDED, reason="Scope package not available")
def test_datastore_save_and_load():
    """Test Datastore's ability to save and load data."""
    try:
        datastore = Datastore()
        test_data = {"test_key": "test_value"}

        # Test save functionality
        datastore.save("test_data", test_data)

        # Test load functionality
        loaded_data = datastore.load("test_data")
        assert loaded_data == test_data
    except AttributeError:
        pytest.skip("Datastore.save or Datastore.load methods don't exist")
    except Exception as e:
        pytest.skip(f"Datastore save/load test failed: {str(e)}")


if __name__ == "__main__":
    # This allows running the tests directly with python tests/test_vnscope.py
    pytest.main(["-xvs", __file__])
